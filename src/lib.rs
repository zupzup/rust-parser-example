/// Example:
///
/// Basic HTTP Parser
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take, take_until, take_while},
    character::{
        complete::{alpha1, alphanumeric0, alphanumeric1, digit1, one_of},
        is_alphanumeric,
    },
    combinator::{cond, opt},
    error::Error,
    error::ErrorKind,
    multi::{count, many0, many1, many_m_n, separated_list1},
    sequence::{pair, separated_pair, terminated, tuple},
    AsChar, Err as NomErr, IResult, InputTakeAtPosition,
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

fn authority(input: &str) -> IResult<&str, Option<(&str, Option<&str>)>> {
    opt(terminated(
        separated_pair(alphanumeric1, opt(tag(":")), opt(alphanumeric1)),
        tag("@"),
    ))(input)
}

// fn host_or_ip(input: &str) -> IResult<&str, String> {}

fn host(input: &str) -> IResult<&str, String> {
    alt((
        tuple((many1(terminated(alphanumerichyphen1, tag("."))), alpha1)),
        tuple((many_m_n(1, 1, alphanumerichyphen1), take(0 as usize))),
    ))(input)
    .and_then(|(next_input, mut res)| {
        println!("res: {:?}", res);
        if !res.1.is_empty() {
            res.0.push(res.1);
        }
        Ok((next_input, res.0.join(".")))
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
            // println!(
            //     "char: {} res: {}",
            //     char_item,
            //     !(char_item == '-') && !char_item.is_alphanum()
            // );
            !(char_item == '-') && !char_item.is_alphanum()
        },
        ErrorKind::AlphaNumeric,
    )
}

// only IPv4
fn ip(input: &str) -> IResult<&str, String> {
    tuple((
        count(terminated(many_m_n(1, 3, one_digit), tag(".")), 3),
        many_m_n(1, 3, one_digit),
    ))(input)
    .and_then(|(next_input, res)| {
        let mut result: Vec<String> = res
            .0
            .into_iter()
            .map(|chars| chars.into_iter().collect::<String>())
            .collect();
        let last: String = res.1.into_iter().collect::<String>();
        result.push(last);
        for num in &result {
            match num.parse::<usize>() {
                Ok(n) => {
                    if n > 255 {
                        return Err(NomErr::Error(Error::new(next_input, ErrorKind::Digit)));
                    }
                }
                Err(_) => return Err(NomErr::Error(Error::new(next_input, ErrorKind::Digit))),
            };
        }
        let ip = result.join(".");

        Ok((next_input, ip))
    })
}

// TODO: parser for up_to_three_digits
// TODO: parser which checks if it's a valid usize and >255 etc.
fn one_digit(input: &str) -> IResult<&str, char> {
    one_of("0123456789")(input)
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
        Ok((":8080", "localhost".to_string()))
    );
    assert_eq!(
        host("example.org:8080"),
        Ok((":8080", "example.org".to_string()))
    );
    assert_eq!(
        host("some-subsite.example.org:8080"),
        Ok((":8080", "some-subsite.example.org".to_string()))
    );
    assert_eq!(host("example.123"), Ok((".123", "example".to_string())));
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
        Ok((":8080", "192.168.0.1".to_string()))
    );
    assert_eq!(ip("0.0.0.0:8080"), Ok((":8080", "0.0.0.0".to_string())));
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
        Ok(("4:8080", "192.168.0.144".to_string()))
    );
    assert_eq!(
        ip("192.168.0:8080"),
        Err(NomErr::Error(Error::new(":8080", ErrorKind::Tag)))
    );
    assert_eq!(
        ip("999.168.0.0:8080"),
        Err(NomErr::Error(Error::new(":8080", ErrorKind::Digit)))
    );
}
