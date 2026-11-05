use crate::protocol::{ProtocolError, Reader, Uuid, put_utf};
use std::collections::HashMap;

/// Players and groups kept; a server that sends more is ignored past this.
const MAX_ENTRIES: usize = 20_000;
pub const MAX_GROUP_NAME: usize = 24;
pub const MAX_GROUP_PASSWORD: usize = 256;

pub const STATE: &str = "voicechat:state";
pub const STATES: &str = "voicechat:states";
pub const REMOVE_STATE: &str = "voicechat:remove_state";
pub const ADD_GROUP: &str = "voicechat:add_group";
pub const REMOVE_GROUP: &str = "voicechat:remove_group";
pub const JOINED_GROUP: &str = "voicechat:joined_group";
pub const CLIENT_STATE: &str = "voicechat:client_state";
pub const CREATE_GROUP: &str = "voicechat:create_group";
pub const SET_GROUP: &str = "voicechat:set_group";
pub const LEAVE_GROUP: &str = "voicechat:leave_group";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayerState {
    pub uuid: Uuid,
    pub name: String,
    pub muted: bool,
    pub force_muted: bool,
    pub disabled: bool,
    pub force_disabled: bool,
    pub disconnected: bool,
    pub group: Option<Uuid>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Badge {
    Disconnected,
    FullMuted,
    Muted,
}

impl PlayerState {
    pub fn badge(&self) -> Option<Badge> {
        if self.disconnected {
            Some(Badge::Disconnected)
        } else if self.disabled || self.force_disabled {
            Some(Badge::FullMuted)
        } else if self.muted || self.force_muted {
            Some(Badge::Muted)
        } else {
            None
        }
    }

    fn read_legacy(r: &mut Reader) -> Result<Self, ProtocolError> {
        let disabled = r.bool_strict()?;
        let disconnected = r.bool_strict()?;
        let uuid = r.uuid()?;
        let name = r.utf(256)?;
        let group = if r.bool_strict()? { Some(r.uuid()?) } else { None };
        Ok(Self {
            uuid,
            name,
            disabled,
            disconnected,
            group,
            ..Self::default()
        })
    }

    fn read(r: &mut Reader) -> Result<Self, ProtocolError> {
        let flags = r.u8()?;
        let uuid = r.uuid()?;
        let name = r.utf(256)?;
        let flag = |bit: u8| flags & (1 << bit) != 0;
        let group = if flag(5) { Some(r.uuid()?) } else { None };
        Ok(Self {
            uuid,
            name,
            muted: flag(0),
            force_muted: flag(1),
            disabled: flag(2),
            force_disabled: flag(3),
            disconnected: flag(4),
            group,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub id: Uuid,
    pub name: String,
    pub has_password: bool,
    pub persistent: bool,
    pub hidden: bool,
    pub kind: i16,
}

impl Group {
    pub fn kind_name(&self) -> &'static str {
        match self.kind {
            1 => "open",
            2 => "isolated",
            _ => "normal",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Incoming {
    State(PlayerState),
    States(Vec<PlayerState>),
    RemoveState(Uuid),
    AddGroup(Group),
    RemoveGroup(Uuid),
    Joined { group: Option<Uuid>, wrong_password: bool },
}

pub fn is_channel(channel: &str) -> bool {
    matches!(
        channel,
        STATE | STATES | REMOVE_STATE | ADD_GROUP | REMOVE_GROUP | JOINED_GROUP
    )
}

pub fn parse(channel: &str, data: &[u8]) -> Result<Incoming, ProtocolError> {
    parse_detect(channel, data).map(|(incoming, _)| incoming)
}

fn read_states(
    data: &[u8],
    read: fn(&mut Reader) -> Result<PlayerState, ProtocolError>,
) -> Result<Vec<PlayerState>, ProtocolError> {
    let mut r = Reader(data);
    let count = r.i32()?;
    if !(0..=100_000).contains(&count) {
        return Err(ProtocolError::Range);
    }
    let mut all = Vec::with_capacity((count as usize).min(r.0.len() / 18 + 1));
    for _ in 0..count {
        all.push(read(&mut r)?);
    }
    if r.0.is_empty() { Ok(all) } else { Err(ProtocolError::Range) }
}

fn read_one(
    data: &[u8],
    read: fn(&mut Reader) -> Result<PlayerState, ProtocolError>,
) -> Result<PlayerState, ProtocolError> {
    let mut r = Reader(data);
    let state = read(&mut r)?;
    if r.0.is_empty() { Ok(state) } else { Err(ProtocolError::Range) }
}

pub fn parse_detect(channel: &str, data: &[u8]) -> Result<(Incoming, bool), ProtocolError> {
    match channel {
        STATE => {
            return match read_one(data, PlayerState::read) {
                Ok(s) => Ok((Incoming::State(s), false)),
                Err(_) => Ok((Incoming::State(read_one(data, PlayerState::read_legacy)?), true)),
            };
        }
        STATES => {
            return match read_states(data, PlayerState::read) {
                Ok(all) => Ok((Incoming::States(all), false)),
                Err(_) => Ok((Incoming::States(read_states(data, PlayerState::read_legacy)?), true)),
            };
        }
        _ => {}
    }
    let mut r = Reader(data);
    Ok((match channel {
        REMOVE_STATE => Incoming::RemoveState(r.uuid()?),
        ADD_GROUP => Incoming::AddGroup(Group {
            id: r.uuid()?,
            name: r.utf(MAX_GROUP_NAME)?,
            has_password: r.bool()?,
            persistent: r.bool()?,
            hidden: r.bool()?,
            kind: r.i16()?,
        }),
        REMOVE_GROUP => Incoming::RemoveGroup(r.uuid()?),
        JOINED_GROUP => {
            let group = if r.bool()? { Some(r.uuid()?) } else { None };
            Incoming::Joined {
                group,
                wrong_password: r.bool()?,
            }
        }
        _ => return Err(ProtocolError::Range),
    }, false))
}

pub fn update_state(muted: bool, disabled: bool) -> Vec<u8> {
    vec![u8::from(muted), u8::from(disabled)]
}

pub fn update_state_legacy(disabled: bool) -> Vec<u8> {
    vec![u8::from(disabled)]
}

pub fn create_group(name: &str, password: Option<&str>, kind: i16) -> Vec<u8> {
    let mut out = Vec::new();
    put_utf(&mut out, name);
    out.push(u8::from(password.is_some()));
    if let Some(p) = password {
        put_utf(&mut out, p);
    }
    out.extend_from_slice(&kind.to_be_bytes());
    out
}

pub fn join_group(id: Uuid, password: Option<&str>) -> Vec<u8> {
    let mut out = id.to_be_bytes().to_vec();
    out.push(u8::from(password.is_some()));
    if let Some(p) = password {
        put_utf(&mut out, p);
    }
    out
}

pub fn leave_group() -> Vec<u8> {
    Vec::new()
}

#[derive(Default)]
pub struct Roster {
    pub players: HashMap<Uuid, PlayerState>,
    pub groups: HashMap<Uuid, Group>,
    pub my_group: Option<Uuid>,
    pub notice: Option<String>,
    pub legacy: bool,
}

impl Roster {
    pub fn apply(&mut self, incoming: Incoming, me: Uuid) {
        match incoming {
            Incoming::State(s) => {
                if s.uuid == me {
                    self.my_group = s.group;
                }
                if self.players.len() < MAX_ENTRIES || self.players.contains_key(&s.uuid) {
                    self.players.insert(s.uuid, s);
                }
            }
            Incoming::States(all) => {
                self.players = all.into_iter().map(|s| (s.uuid, s)).collect();
                self.my_group = self.players.get(&me).and_then(|mine| mine.group);
            }
            Incoming::RemoveState(id) => {
                self.players.remove(&id);
            }
            Incoming::AddGroup(g) => {
                if self.groups.len() < MAX_ENTRIES || self.groups.contains_key(&g.id) {
                    self.groups.insert(g.id, g);
                }
            }
            Incoming::RemoveGroup(id) => {
                self.groups.remove(&id);
                if self.my_group == Some(id) {
                    self.my_group = None;
                }
            }
            Incoming::Joined { group, wrong_password } => {
                if wrong_password {
                    self.notice = Some("Wrong group password.".into());
                } else {
                    self.my_group = group;
                }
            }
        }
    }

    pub fn find_group(&self, text: &str) -> Option<&Group> {
        let text = text.trim();
        let mut sorted: Vec<&Group> = self.groups.values().filter(|g| !g.hidden).collect();
        sorted.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        if let Ok(n) = text.parse::<usize>() {
            if n >= 1 {
                return sorted.get(n - 1).copied();
            }
        }
        sorted
            .iter()
            .copied()
            .find(|g| g.name.eq_ignore_ascii_case(text))
            .or_else(|| {
                sorted
                    .iter()
                    .copied()
                    .find(|g| g.name.to_lowercase().starts_with(&text.to_lowercase()))
            })
    }

    pub fn listed_groups(&self) -> Vec<&Group> {
        let mut v: Vec<&Group> = self.groups.values().filter(|g| !g.hidden).collect();
        v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        v
    }

    pub fn members(&self, group: Uuid) -> usize {
        self.players.values().filter(|p| p.group == Some(group)).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_state(out: &mut Vec<u8>, flags: u8, uuid: u128, name: &str, group: Option<u128>) {
        out.push(flags);
        out.extend_from_slice(&uuid.to_be_bytes());
        put_utf(out, name);
        if let Some(g) = group {
            out.extend_from_slice(&g.to_be_bytes());
        }
    }

    #[test]
    fn a_player_state_reads_its_flags_and_group() {
        let mut data = Vec::new();
        put_state(&mut data, 0b10_0101, 7, "Alex", Some(9));
        let Incoming::State(s) = parse(STATE, &data).unwrap() else {
            panic!()
        };
        assert_eq!((s.uuid, s.name.as_str(), s.group), (7, "Alex", Some(9)));
        assert!(s.muted && s.disabled && !s.disconnected && !s.force_muted);
        assert_eq!(s.badge(), Some(Badge::FullMuted));
    }

    #[test]
    fn badges_rank_disconnected_over_full_mute_over_mute() {
        let mut s = PlayerState::default();
        assert_eq!(s.badge(), None);
        s.muted = true;
        assert_eq!(s.badge(), Some(Badge::Muted));
        s.force_disabled = true;
        assert_eq!(s.badge(), Some(Badge::FullMuted));
        s.disconnected = true;
        assert_eq!(s.badge(), Some(Badge::Disconnected));
    }

    fn hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn the_older_two_bool_format_a_real_server_sent_is_read() {
        let state = hex("0000141347b887164752b1b05a2e4f1d76a60a4d63436c61756465414900");
        let (Incoming::State(s), legacy) = parse_detect(STATE, &state).unwrap() else {
            panic!()
        };
        assert!(legacy);
        assert_eq!(s.name, "McClaudeAI");
        assert_eq!(s.uuid, 0x141347b8_8716_4752_b1b0_5a2e4f1d76a6);
        assert!(!s.disabled && !s.disconnected && s.group.is_none());
        let mut many = 2i32.to_be_bytes().to_vec();
        many.extend_from_slice(&hex("01004e3a6b1400fa4690b52d5aaac7812e0609466c65726f32343734"));
        many.push(0);
        many.extend_from_slice(&state);
        let (Incoming::States(all), legacy) = parse_detect(STATES, &many).unwrap() else {
            panic!()
        };
        assert!(legacy);
        assert_eq!(all.len(), 2);
        assert!(all[0].disabled);
        assert_eq!(all[0].name, "Flero2474");
        assert_eq!(update_state_legacy(true), vec![1]);
    }

    #[test]
    fn the_states_list_has_a_four_byte_count() {
        let mut data = 2i32.to_be_bytes().to_vec();
        put_state(&mut data, 0, 1, "A", None);
        put_state(&mut data, 0b1_0000, 2, "B", None);
        let Incoming::States(all) = parse(STATES, &data).unwrap() else {
            panic!()
        };
        assert_eq!(all.len(), 2);
        assert!(all[1].disconnected);
        assert!(parse(STATES, &(-1i32).to_be_bytes()).is_err());
        assert!(parse(STATES, &data[..10]).is_err());
    }

    #[test]
    fn groups_and_joins_decode() {
        let mut data = 5u128.to_be_bytes().to_vec();
        put_utf(&mut data, "Team");
        data.extend_from_slice(&[1, 0, 0]);
        data.extend_from_slice(&1i16.to_be_bytes());
        let Incoming::AddGroup(g) = parse(ADD_GROUP, &data).unwrap() else {
            panic!()
        };
        assert_eq!((g.name.as_str(), g.has_password, g.kind_name()), ("Team", true, "open"));
        let mut joined = vec![1];
        joined.extend_from_slice(&5u128.to_be_bytes());
        joined.push(0);
        assert_eq!(
            parse(JOINED_GROUP, &joined).unwrap(),
            Incoming::Joined { group: Some(5), wrong_password: false }
        );
        assert_eq!(
            parse(JOINED_GROUP, &[0, 1]).unwrap(),
            Incoming::Joined { group: None, wrong_password: true }
        );
        assert_eq!(parse(REMOVE_STATE, &3u128.to_be_bytes()).unwrap(), Incoming::RemoveState(3));
    }

    #[test]
    fn outgoing_packets_have_the_documented_layout() {
        assert_eq!(update_state(true, false), vec![1, 0]);
        let create = create_group("Hi", Some("pw"), 1);
        assert_eq!(create, [&[2][..], b"Hi", &[1, 2], b"pw", &[0, 1]].concat());
        let join = join_group(1, None);
        assert_eq!(join.len(), 17);
        assert_eq!(join[16], 0);
        assert!(leave_group().is_empty());
    }

    #[test]
    fn the_roster_follows_the_servers_messages() {
        let mut r = Roster::default();
        let g = |id, name: &str| Group {
            id,
            name: name.into(),
            has_password: false,
            persistent: false,
            hidden: false,
            kind: 0,
        };
        r.apply(Incoming::AddGroup(g(1, "Beta")), 10);
        r.apply(Incoming::AddGroup(g(2, "alpha")), 10);
        assert_eq!(r.find_group("1").unwrap().name, "alpha");
        assert_eq!(r.find_group("BETA").unwrap().id, 1);
        assert_eq!(r.find_group("al").unwrap().id, 2);
        assert!(r.find_group("9").is_none());
        r.apply(Incoming::Joined { group: Some(1), wrong_password: false }, 10);
        assert_eq!(r.my_group, Some(1));
        r.apply(Incoming::Joined { group: None, wrong_password: true }, 10);
        assert_eq!(r.my_group, Some(1));
        assert!(r.notice.is_some());
        r.apply(Incoming::RemoveGroup(1), 10);
        assert_eq!(r.my_group, None);
        let st = PlayerState { uuid: 10, group: Some(2), ..Default::default() };
        r.apply(Incoming::State(st), 10);
        assert_eq!(r.my_group, Some(2));
        assert_eq!(r.members(2), 1);
    }
}
