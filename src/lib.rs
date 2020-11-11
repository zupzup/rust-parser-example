/// Example:
///
/// Basic HTTP Parser
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take, take_until, take_while},
    character::{
        complete::{alpha1, alphanumeric0, alphanumeric1, digit1, newline, one_of, space0},
        is_alphanumeric, is_newline,
    },
    combinator::{cond, opt},
    error::Error,
    error::ErrorKind,
    multi::{count, many0, many1, many_m_n, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    AsChar, Err as NomErr, IResult, InputTakeAtPosition,
};

type Header = (String, String);
type Headers = Vec<Header>;

#[derive(Debug, PartialEq, Eq)]
enum Method {
    GET,
    HEAD,
    POST,
    PUT,
    DELETE,
    CONNECT,
    OPTIONS,
    TRACE,
}

#[derive(Debug, PartialEq, Eq)]
enum Host {
    HOST(String),
    IP([u8; 4]),
    ASTERISK,
}

impl From<&str> for Method {
    fn from(i: &str) -> Self {
        match i.to_uppercase().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "HEAD" => Method::HEAD,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            "CONNECT" => Method::CONNECT,
            "OPTIONS" => Method::OPTIONS,
            "TRACE" => Method::TRACE,
            _ => unimplemented!("There are no other request methods"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Scheme {
    HTTP,
    HTTPS,
}

impl From<&str> for Scheme {
    fn from(i: &str) -> Self {
        match i.to_uppercase().as_str() {
            "http" => Scheme::HTTP,
            "https" => Scheme::HTTPS,
            _ => unimplemented!("no other schemes supported"),
        }
    }
}

/// Based on https://url.spec.whatwg.org/#urls
struct URI {
    scheme: Scheme,
    authority: Option<(Option<String>, Option<String>)>, // username & password
    host: Host,
    port: Option<u16>,
    path: Option<String>,
    query: Option<Vec<(String, String)>>,
    fragment: Option<String>,
}

struct Request {
    method: Method,
    uri: URI,
}

fn scheme(input: &str) -> IResult<&str, Scheme> {
    alt((tag_no_case("HTTP://"), tag_no_case("HTTPS://")))(input)
        .and_then(|(next_input, res)| Ok((next_input, res.into())))
}

fn authority(input: &str) -> IResult<&str, Option<(&str, Option<&str>)>> {
    opt(terminated(
        separated_pair(alphanumeric1, opt(tag(":")), opt(alphanumeric1)),
        tag("@"),
    ))(input)
}

fn host(input: &str) -> IResult<&str, Host> {
    alt((
        tuple((many1(terminated(alphanumerichyphen1, tag("."))), alpha1)),
        tuple((many_m_n(1, 1, alphanumerichyphen1), take(0 as usize))),
    ))(input)
    .and_then(|(next_input, mut res)| {
        println!("res: {:?}", res);
        if !res.1.is_empty() {
            res.0.push(res.1);
        }
        Ok((next_input, Host::HOST(res.0.join("."))))
    })
}

fn alphanumerichyphen1<T>(i: T) -> IResult<T, T>
where
    T: InputTakeAtPosition,
    <T as InputTakeAtPosition>::Item: AsChar,
{
    i.split_at_position1_complete(
        |item| {
            let char_item = item.as_char();
            !(char_item == '-') && !char_item.is_alphanum()
        },
        ErrorKind::AlphaNumeric,
    )
}

fn host_asterisk(input: &str) -> IResult<&str, Host> {
    tag("*")(input).and_then(|(next_input, res)| Ok((next_input, Host::ASTERISK)))
}

// only IPv4
fn ip(input: &str) -> IResult<&str, Host> {
    tuple((count(terminated(ip_num, tag(".")), 3), ip_num))(input).and_then(|(next_input, res)| {
        let mut result: [u8; 4] = [0, 0, 0, 0];
        res.0
            .into_iter()
            .enumerate()
            .for_each(|(i, v)| result[i] = v);
        result[3] = res.1;
        Ok((next_input, Host::IP(result)))
    })
}

fn ip_num(input: &str) -> IResult<&str, u8> {
    one_to_three_digits(input).and_then(|(next_input, result)| match result.parse::<u8>() {
        Ok(n) => Ok((next_input, n)),
        Err(_) => Err(NomErr::Error(Error::new(next_input, ErrorKind::Digit))), // TODO: use https://docs.rs/nom/6.0.0/nom/error/index.html to add error context
    })
}

fn version(input: &str) -> IResult<&str, &str> {
    tag("HTTP/1.1")(input)
}

fn headers(input: &str) -> IResult<&str, Headers> {
    many0(header)(input)
}

fn header(input: &str) -> IResult<&str, Header> {
    separated_pair(
        alphanumerichyphen1,
        spaced_colon,
        terminated(take_while(not_newline), newline),
    )(input)
    .and_then(|(next_input, res)| Ok((next_input, (res.0.to_owned(), res.1.to_owned()))))
}

fn spaced_colon(input: &str) -> IResult<&str, &str> {
    delimited(space0, tag(":"), space0)(input)
}

fn not_newline(chr: char) -> bool {
    chr != '\n'
}

// TODO: n to m digits

fn one_to_three_digits(input: &str) -> IResult<&str, String> {
    many_m_n(1, 3, one_digit)(input)
        .and_then(|(next_input, result)| Ok((next_input, result.into_iter().collect())))
}

fn two_to_four_digits(input: &str) -> IResult<&str, String> {
    many_m_n(2, 4, one_digit)(input)
        .and_then(|(next_input, result)| Ok((next_input, result.into_iter().collect())))
}

fn one_digit(input: &str) -> IResult<&str, char> {
    one_of("0123456789")(input)
}

fn host_or_ip(input: &str) -> IResult<&str, Host> {
    alt((host, ip))(input)
}

fn port(input: &str) -> IResult<&str, u16> {
    preceded(tag(":"), two_to_four_digits)(input).and_then(|(next_input, res)| {
        match res.parse::<u16>() {
            Ok(n) => Ok((next_input, n)),
            Err(_) => Err(NomErr::Error(Error::new(next_input, ErrorKind::Digit))), // TODO: use https://docs.rs/nom/6.0.0/nom/error/index.html to add error context
        }
    })
}

fn uri(input: &str) -> IResult<&str, URI> {
    // TODO: optional (optional (scheme, authority, host or ip, port), relative path and query), or *
}

fn request_method(input: &str) -> IResult<&str, Method> {
    alt((
        tag_no_case("GET"),
        tag_no_case("POST"),
        tag_no_case("PUT"),
        tag_no_case("DELETE"),
        tag_no_case("CONNECT"),
        tag_no_case("OPTIONS"),
        tag_no_case("TRACE"),
    ))(input)
    .and_then(|(next_input, res)| Ok((next_input, res.into())))
}
/// REQUEST LINE: https://tools.ietf.org/html/rfc7230#section-3.1.1
// fn parse_http(input: &str) -> IResult<&str, Request> {
//     Ok(("", Request {}))
// }

#[test]
fn test_request_method() {
    assert_eq!(request_method("GET 1234"), Ok((" 1234", Method::GET)));
    assert_eq!(
        request_method("1234"),
        Err(NomErr::Error(Error::new("1234", ErrorKind::Tag)))
    );
    assert_eq!(request_method("PUT POST"), Ok((" POST", Method::PUT)));
}

#[test]
fn test_authority() {
    assert_eq!(
        authority("username:password@zupzup.org"),
        Ok(("zupzup.org", Some(("username", Some("password")))))
    );
    assert_eq!(
        authority("username@zupzup.org"),
        Ok(("zupzup.org", Some(("username", None))))
    );
    assert_eq!(authority("zupzup.org"), Ok(("zupzup.org", None)));
    assert_eq!(authority(":zupzup.org"), Ok((":zupzup.org", None)));
    assert_eq!(
        authority("username:passwordzupzup.org"),
        Ok(("username:passwordzupzup.org", None))
    );
    assert_eq!(authority("@zupzup.org"), Ok(("@zupzup.org", None)));
}

#[test]
fn test_host() {
    assert_eq!(
        host("localhost:8080"),
        Ok((":8080", Host::HOST("localhost".to_string())))
    );
    assert_eq!(
        host("example.org:8080"),
        Ok((":8080", Host::HOST("example.org".to_string())))
    );
    assert_eq!(
        host("some-subsite.example.org:8080"),
        Ok((":8080", Host::HOST("some-subsite.example.org".to_string())))
    );
    assert_eq!(
        host("example.123"),
        Ok((".123", Host::HOST("example".to_string())))
    );
    assert_eq!(
        host("$$$.com"),
        Err(NomErr::Error(Error::new(
            "$$$.com",
            ErrorKind::AlphaNumeric
        )))
    );
    assert_eq!(
        host(".com"),
        Err(NomErr::Error(Error::new(".com", ErrorKind::AlphaNumeric)))
    );
}

#[test]
fn test_ipv4() {
    assert_eq!(
        ip("192.168.0.1:8080"),
        Ok((":8080", Host::IP([192, 168, 0, 1])))
    );
    assert_eq!(ip("0.0.0.0:8080"), Ok((":8080", Host::IP([0, 0, 0, 0]))));
    assert_eq!(
        ip("1924.168.0.1:8080"),
        Err(NomErr::Error(Error::new("4.168.0.1:8080", ErrorKind::Tag)))
    );
    assert_eq!(
        ip("192.168.0000.144:8080"),
        Err(NomErr::Error(Error::new("0.144:8080", ErrorKind::Tag)))
    );
    assert_eq!(
        ip("192.168.0.1444:8080"),
        Ok(("4:8080", Host::IP([192, 168, 0, 144])))
    );
    assert_eq!(
        ip("192.168.0:8080"),
        Err(NomErr::Error(Error::new(":8080", ErrorKind::Tag)))
    );
    assert_eq!(
        ip("999.168.0.0:8080"),
        Err(NomErr::Error(Error::new(".168.0.0:8080", ErrorKind::Digit)))
    );
}

#[test]
fn test_header() {
    assert_eq!(
        header("Content-Type: application/json\nabc"),
        Ok((
            "abc",
            ("Content-Type".to_string(), "application/json".to_string())
        ))
    );
    assert_eq!(
        header("Content-Type  :          application/json\nabc"),
        Ok((
            "abc",
            ("Content-Type".to_string(), "application/json".to_string())
        ))
    );
    assert_eq!(
        header("Some1:123$$$%*%*\nabc"),
        Ok(("abc", ("Some1".to_string(), "123$$$%*%*".to_string())))
    );
}

#[test]
fn test_headers() {
    assert_eq!(
        headers("Content-Type: application/json\nAuthorization: pw\nHost: zupzup.org\nabc"),
        Ok((
            "abc",
            vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Authorization".to_string(), "pw".to_string()),
                ("Host".to_string(), "zupzup.org".to_string())
            ]
        ))
    );
}
