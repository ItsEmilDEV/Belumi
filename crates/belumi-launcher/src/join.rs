//! Join links: `belumi://join/play.example.org:25565` starts the game on that server. The launcher asks before it joins, as
//! a link on a web page can come from anyone.

const SCHEME: &str = "belumi://";

/// The server address of a join link, when the text is one and the address is plausible (a host name or an IPv4 address with
/// an optional port, or an IPv6 address in brackets with an optional port).
pub fn parse(link: &str) -> Option<String> {
    let rest = link.trim().get(..SCHEME.len()).filter(|s| s.eq_ignore_ascii_case(SCHEME)).map(|_| &link.trim()[SCHEME.len()..])?;
    let address = rest.strip_prefix("join/")?.trim_end_matches('/');
    let address = decode(address)?;
    valid_address(&address).then_some(address)
}

/// The link that joins `address`.
#[cfg(test)]
pub fn link(address: &str) -> String {
    format!("{SCHEME}join/{}", address.trim())
}

/// `%3A` and the like back to characters; anything that is not a plain address character stays refused later.
fn decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn valid_port(p: &str) -> bool {
    !p.is_empty() && p.len() <= 5 && p.bytes().all(|b| b.is_ascii_digit()) && p.parse::<u32>().is_ok_and(|n| (1..=65535).contains(&n))
}

fn valid_host(h: &str) -> bool {
    !h.is_empty()
        && h.len() <= 253
        && !h.starts_with(['.', '-'])
        && !h.ends_with(['.', '-'])
        && !h.contains("..")
        && h.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'_')
}

pub fn valid_address(address: &str) -> bool {
    if address.len() > 255 {
        return false;
    }
    if let Some(rest) = address.strip_prefix('[') {
        let Some((v6, tail)) = rest.split_once(']') else { return false };
        let ok_v6 = !v6.is_empty() && v6.bytes().all(|b| b.is_ascii_hexdigit() || b == b':' || b == b'.');
        return ok_v6 && (tail.is_empty() || tail.strip_prefix(':').is_some_and(valid_port));
    }
    match address.split_once(':') {
        Some((host, port)) => valid_host(host) && valid_port(port),
        None => valid_host(address),
    }
}

/// The first join link among the program's arguments.
pub fn from_args(args: impl IntoIterator<Item = String>) -> Option<String> {
    args.into_iter().find_map(|a| parse(&a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_links_give_their_address() {
        assert_eq!(parse("belumi://join/play.example.org").as_deref(), Some("play.example.org"));
        assert_eq!(parse("belumi://join/play.example.org:25566/").as_deref(), Some("play.example.org:25566"));
        assert_eq!(parse("BELUMI://join/10.0.0.5:25565").as_deref(), Some("10.0.0.5:25565"));
        assert_eq!(parse("  belumi://join/mc-1.example.org  ").as_deref(), Some("mc-1.example.org"));
        assert_eq!(parse("belumi://join/%5B::1%5D:25565").as_deref(), Some("[::1]:25565"));
        assert_eq!(parse("belumi://join/[2001:db8::1]").as_deref(), Some("[2001:db8::1]"));
    }

    #[test]
    fn anything_else_is_refused() {
        for bad in [
            "https://join/play.example.org",
            "belumi://open/play.example.org",
            "belumi://join/",
            "belumi://join/play example.org",
            "belumi://join/play.example.org:0",
            "belumi://join/play.example.org:99999",
            "belumi://join/play.example.org:abc",
            "belumi://join/-bad.example.org",
            "belumi://join/a..b",
            "belumi://join/host/extra/path",
            "belumi://join/host;rm",
            "belumi://join/%zz",
            "belumi://join/[::1",
            "belumi://join/user@host",
            "--server",
            "",
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
        assert_eq!(parse(&format!("belumi://join/{}", "a".repeat(300))), None);
    }

    #[test]
    fn a_link_made_from_an_address_reads_back() {
        for a in ["play.example.org", "play.example.org:25566", "[::1]:25565"] {
            assert_eq!(parse(&link(a)).as_deref(), Some(a));
        }
    }

    #[test]
    fn the_first_link_among_the_arguments_counts() {
        let args = vec!["--x".to_owned(), "belumi://join/a.example.org".to_owned(), "belumi://join/b.example.org".to_owned()];
        assert_eq!(from_args(args).as_deref(), Some("a.example.org"));
        assert_eq!(from_args(vec!["--play".to_owned()]), None);
    }
}
