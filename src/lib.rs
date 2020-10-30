/// Example:
///
/// Basic HTTP Parser
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_until, take_while},
    character::complete::{alphanumeric0, alphanumeric1},
    combinator::{cond, opt},
    error::ErrorKind,
    multi::many1,
    sequence::{pair, separated_pair, terminated},
    Err as NomErr, IResult,
};

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
    host: String,
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

fn username(input: &str) -> IResult<&str, &str> {
    alt((take_until(":"), take_until("@")))(input)
}

fn password(input: &str) -> IResult<&str, &str> {
    take_until("@")(input)
}

fn authority(input: &str) -> IResult<&str, Option<(&str, Option<&str>)>> {
    opt(terminated(
        separated_pair(alphanumeric1, opt(tag(":")), opt(alphanumeric1)),
        tag("@"),
    ))(input)
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

// fn parse_http(input: &str) -> IResult<&str, Request> {
//     Ok(("", Request {}))
// }

#[test]
fn test_request_method() {
    assert_eq!(request_method("GET 1234"), Ok((" 1234", Method::GET)));
    assert_eq!(
        request_method("1234"),
        Err(NomErr::Error(("1234", ErrorKind::Tag)))
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
