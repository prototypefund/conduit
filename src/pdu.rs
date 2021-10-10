use crate::Error;
use ruma::{
    events::{
        pdu::EventHash, room::member::MemberEventContent, AnyEphemeralRoomEvent,
        AnyInitialStateEvent, AnyRoomEvent, AnyStateEvent, AnyStrippedStateEvent, AnySyncRoomEvent,
        AnySyncStateEvent, EventType, StateEvent,
    },
    serde::{CanonicalJsonObject, CanonicalJsonValue, Raw},
    state_res, EventId, MilliSecondsSinceUnixEpoch, RoomId, RoomVersionId, ServerName, UInt,
    UserId,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue};
use std::{cmp::Ordering, collections::BTreeMap, convert::TryFrom, sync::RwLock};
use tracing::warn;

#[derive(Deserialize, Serialize, Debug)]
pub struct PduEvent {
    pub event_id: EventId,
    pub room_id: RoomId,
    pub sender: UserId,
    pub origin_server_ts: UInt,
    #[serde(rename = "type")]
    pub kind: EventType,
    pub content: Box<serde_json::value::RawValue>,
    #[serde(skip)]
    pub parsed_content: RwLock<Option<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_key: Option<String>,
    pub prev_events: Vec<EventId>,
    pub depth: UInt,
    pub auth_events: Vec<EventId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacts: Option<EventId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unsigned: Option<Box<serde_json::value::RawValue>>,
    pub hashes: EventHash,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signatures: Option<Box<serde_json::value::RawValue>>, // BTreeMap<Box<ServerName>, BTreeMap<ServerSigningKeyId, String>>
}

impl PduEvent {
    #[tracing::instrument(skip(self))]
    pub fn redact(&mut self, reason: &PduEvent) -> crate::Result<()> {
        self.unsigned = None;

        let allowed: &[&str] = match self.kind {
            EventType::RoomMember => &["membership"],
            EventType::RoomCreate => &["creator"],
            EventType::RoomJoinRules => &["join_rule"],
            EventType::RoomPowerLevels => &[
                "ban",
                "events",
                "events_default",
                "kick",
                "redact",
                "state_default",
                "users",
                "users_default",
            ],
            EventType::RoomHistoryVisibility => &["history_visibility"],
            _ => &[],
        };

        let mut old_content =
            serde_json::from_str::<BTreeMap<String, serde_json::Value>>(self.content.get())
                .map_err(|_| Error::bad_database("PDU in db has invalid content."))?;

        let mut new_content = serde_json::Map::new();

        for key in allowed {
            if let Some(value) = old_content.remove(*key) {
                new_content.insert((*key).to_owned(), value);
            }
        }

        self.unsigned = Some(RawValue::from_string(serde_json::to_string(&json!({
            "redacted_because": serde_json::to_value(reason).expect("to_value(PduEvent) always works")
        })).expect("to string always works")).expect("string is valid"));

        self.content = RawValue::from_string(
            serde_json::to_string(&new_content).expect("to string always works"),
        )
        .expect("string is valid");

        Ok(())
    }

    pub fn remove_transaction_id(&mut self) -> crate::Result<()> {
        if let Some(unsigned) = &self.unsigned {
            let mut unsigned =
                serde_json::from_str::<BTreeMap<String, Box<RawValue>>>(unsigned.get())
                    .map_err(|_| Error::bad_database("Invalid unsigned in pdu event"))?;
            unsigned.remove("transaction_id");
            self.unsigned = Some(
                RawValue::from_string(serde_json::to_string(&unsigned).expect("unsigned is valid"))
                    .expect("string is valid"),
            );
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn to_sync_room_event(&self) -> Raw<AnySyncRoomEvent> {
        let mut json = json!({
            "content": self.content,
            "type": self.kind,
            "event_id": self.event_id,
            "sender": self.sender,
            "origin_server_ts": self.origin_server_ts,
            "unsigned": self.unsigned,
        });

        if let Some(state_key) = &self.state_key {
            json["state_key"] = json!(state_key);
        }
        if let Some(redacts) = &self.redacts {
            json["redacts"] = json!(redacts);
        }

        serde_json::from_value(json).expect("Raw::from_value always works")
    }

    /// This only works for events that are also AnyRoomEvents.
    #[tracing::instrument(skip(self))]
    pub fn to_any_event(&self) -> Raw<AnyEphemeralRoomEvent> {
        let mut json = json!({
            "content": self.content,
            "type": self.kind,
            "event_id": self.event_id,
            "sender": self.sender,
            "origin_server_ts": self.origin_server_ts,
            "unsigned": self.unsigned,
            "room_id": self.room_id,
        });

        if let Some(state_key) = &self.state_key {
            json["state_key"] = json!(state_key);
        }
        if let Some(redacts) = &self.redacts {
            json["redacts"] = json!(redacts);
        }

        serde_json::from_value(json).expect("Raw::from_value always works")
    }

    #[tracing::instrument(skip(self))]
    pub fn to_room_event(&self) -> Raw<AnyRoomEvent> {
        let mut json = json!({
            "content": self.content,
            "type": self.kind,
            "event_id": self.event_id,
            "sender": self.sender,
            "origin_server_ts": self.origin_server_ts,
            "unsigned": self.unsigned,
            "room_id": self.room_id,
        });

        if let Some(state_key) = &self.state_key {
            json["state_key"] = json!(state_key);
        }
        if let Some(redacts) = &self.redacts {
            json["redacts"] = json!(redacts);
        }

        serde_json::from_value(json).expect("Raw::from_value always works")
    }

    #[tracing::instrument(skip(self))]
    pub fn to_state_event(&self) -> Raw<AnyStateEvent> {
        let json = json!({
            "content": self.content,
            "type": self.kind,
            "event_id": self.event_id,
            "sender": self.sender,
            "origin_server_ts": self.origin_server_ts,
            "unsigned": self.unsigned,
            "room_id": self.room_id,
            "state_key": self.state_key,
        });

        serde_json::from_value(json).expect("Raw::from_value always works")
    }

    #[tracing::instrument(skip(self))]
    pub fn to_sync_state_event(&self) -> Raw<AnySyncStateEvent> {
        let json = json!({
            "content": self.content,
            "type": self.kind,
            "event_id": self.event_id,
            "sender": self.sender,
            "origin_server_ts": self.origin_server_ts,
            "unsigned": self.unsigned,
            "state_key": self.state_key,
        });

        serde_json::from_value(json).expect("Raw::from_value always works")
    }

    #[tracing::instrument(skip(self))]
    pub fn to_stripped_state_event(&self) -> Raw<AnyStrippedStateEvent> {
        let json = json!({
            "content": self.content,
            "type": self.kind,
            "sender": self.sender,
            "state_key": self.state_key,
        });

        serde_json::from_value(json).expect("Raw::from_value always works")
    }

    #[tracing::instrument(skip(self))]
    pub fn to_member_event(&self) -> Raw<StateEvent<MemberEventContent>> {
        let json = json!({
            "content": self.content,
            "type": self.kind,
            "event_id": self.event_id,
            "sender": self.sender,
            "origin_server_ts": self.origin_server_ts,
            "redacts": self.redacts,
            "unsigned": self.unsigned,
            "room_id": self.room_id,
            "state_key": self.state_key,
        });

        serde_json::from_value(json).expect("Raw::from_value always works")
    }

    /// This does not return a full `Pdu` it is only to satisfy ruma's types.
    #[tracing::instrument]
    pub fn convert_to_outgoing_federation_event(
        mut pdu_json: CanonicalJsonObject,
    ) -> Raw<ruma::events::pdu::Pdu> {
        if let Some(unsigned) = pdu_json
            .get_mut("unsigned")
            .and_then(|val| val.as_object_mut())
        {
            unsigned.remove("transaction_id");
        }

        pdu_json.remove("event_id");

        // TODO: another option would be to convert it to a canonical string to validate size
        // and return a Result<Raw<...>>
        // serde_json::from_str::<Raw<_>>(
        //     ruma::serde::to_canonical_json_string(pdu_json).expect("CanonicalJson is valid serde_json::Value"),
        // )
        // .expect("Raw::from_value always works")

        serde_json::from_value::<Raw<_>>(
            serde_json::to_value(pdu_json).expect("CanonicalJson is valid serde_json::Value"),
        )
        .expect("Raw::from_value always works")
    }

    pub fn from_id_val(
        event_id: &EventId,
        mut json: CanonicalJsonObject,
    ) -> Result<Self, serde_json::Error> {
        json.insert(
            "event_id".to_string(),
            CanonicalJsonValue::String(event_id.as_str().to_owned()),
        );

        serde_json::from_value(serde_json::to_value(json).expect("valid JSON"))
    }
}

impl state_res::Event for PduEvent {
    fn event_id(&self) -> &EventId {
        &self.event_id
    }

    fn room_id(&self) -> &RoomId {
        &self.room_id
    }

    fn sender(&self) -> &UserId {
        &self.sender
    }

    fn event_type(&self) -> &EventType {
        &self.kind
    }

    fn content(&self) -> serde_json::Value {
        self.parsed_content
            .write()
            .unwrap()
            .get_or_insert_with(|| {
                serde_json::to_value(&self.content).expect("content is valid json")
            })
            .clone()
    }

    fn origin_server_ts(&self) -> MilliSecondsSinceUnixEpoch {
        MilliSecondsSinceUnixEpoch(self.origin_server_ts)
    }

    fn state_key(&self) -> Option<&str> {
        self.state_key.as_deref()
    }

    fn prev_events(&self) -> Box<dyn DoubleEndedIterator<Item = &EventId> + '_> {
        Box::new(self.prev_events.iter())
    }

    fn depth(&self) -> &UInt {
        &self.depth
    }

    fn auth_events(&self) -> Box<dyn DoubleEndedIterator<Item = &EventId> + '_> {
        Box::new(self.auth_events.iter())
    }

    fn redacts(&self) -> Option<&EventId> {
        self.redacts.as_ref()
    }

    fn hashes(&self) -> &EventHash {
        &self.hashes
    }

    fn signatures(&self) -> BTreeMap<Box<ServerName>, BTreeMap<ruma::ServerSigningKeyId, String>> {
        self.signatures
            .as_ref()
            .map(|raw| serde_json::from_str(raw.get()).expect("string is valid signatures json"))
            .unwrap_or_default()
    }

    fn unsigned(&self) -> BTreeMap<String, serde_json::Value> {
        self.unsigned
            .as_ref()
            .map(|raw| serde_json::from_str(raw.get()).expect("string is valid json"))
            .unwrap_or_default()
    }
}

// These impl's allow us to dedup state snapshots when resolving state
// for incoming events (federation/send/{txn}).
impl Eq for PduEvent {}
impl PartialEq for PduEvent {
    fn eq(&self, other: &Self) -> bool {
        self.event_id == other.event_id
    }
}
impl PartialOrd for PduEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.event_id.partial_cmp(&other.event_id)
    }
}
impl Ord for PduEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.event_id.cmp(&other.event_id)
    }
}

/// Generates a correct eventId for the incoming pdu.
///
/// Returns a tuple of the new `EventId` and the PDU as a `BTreeMap<String, CanonicalJsonValue>`.
pub(crate) fn gen_event_id_canonical_json(
    pdu: &Raw<ruma::events::pdu::Pdu>,
) -> crate::Result<(EventId, CanonicalJsonObject)> {
    let value = serde_json::from_str(pdu.json().get()).map_err(|e| {
        warn!("Error parsing incoming event {:?}: {:?}", pdu, e);
        Error::BadServerResponse("Invalid PDU in server response")
    })?;

    let event_id = EventId::try_from(&*format!(
        "${}",
        // Anything higher than version3 behaves the same
        ruma::signatures::reference_hash(&value, &RoomVersionId::Version6)
            .expect("ruma can calculate reference hashes")
    ))
    .expect("ruma's reference hashes are valid event ids");

    Ok((event_id, value))
}

/// Build the start of a PDU in order to add it to the `Database`.
#[derive(Debug, Deserialize)]
pub struct PduBuilder {
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub content: Box<serde_json::value::RawValue>,
    pub unsigned: Option<BTreeMap<String, serde_json::Value>>,
    pub state_key: Option<String>,
    pub redacts: Option<EventId>,
}

/// Direct conversion prevents loss of the empty `state_key` that ruma requires.
impl From<AnyInitialStateEvent> for PduBuilder {
    fn from(event: AnyInitialStateEvent) -> Self {
        Self {
            event_type: EventType::from(event.event_type()),
            content: RawValue::from_string(
                serde_json::to_string(&event.content()).expect(
                    "AnyStateEventContent came from JSON and can thus turn back into JSON.",
                ),
            )
            .expect("string is valid"),
            unsigned: None,
            state_key: Some(event.state_key().to_owned()),
            redacts: None,
        }
    }
}
