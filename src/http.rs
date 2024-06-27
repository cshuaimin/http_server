use std::{
    collections::HashMap,
    fmt::Display,
    fs,
    io::{BufRead, Read},
    mem,
    path::Path,
};

use crate::error::{Error, Result};

#[derive(Debug, PartialEq)]
pub enum Method {
    Get,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Version {
    Http1_0,
    Http1_1,
}

#[derive(Debug, PartialEq)]
pub struct Request {
    pub method: Method,
    pub uri: String,
    pub version: Version,
    // HTTP headers can have same keys.
    pub headers: HashMap<String, Vec<String>>,
}

#[derive(Debug, PartialEq)]
pub struct Response {
    pub version: Version,
    pub status: u16,
    pub reason: String,
    pub headers: HashMap<String, Vec<String>>,
    pub body: Option<String>,
}

impl Method {
    fn parse(s: &str) -> Result<Self> {
        match s {
            "GET" => Ok(Method::Get),
            s => Err(Error::MethodNotSupported(s.into())),
        }
    }
}

impl Version {
    fn parse(s: &str) -> Result<Self> {
        match s {
            "HTTP/1.0" => Ok(Version::Http1_0),
            "HTTP/1.1" => Ok(Version::Http1_1),
            s => Err(Error::HttpVersionNotSupported(s.into())),
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Version::Http1_0 => "HTTP/1.0",
            Version::Http1_1 => "HTTP/1.1",
        };
        write!(f, "{s}",)
    }
}

impl Request {
    pub fn parse(mut reader: impl BufRead, buf: &mut String) -> Result<Request> {
        buf.clear();
        if reader.read_line(buf)? == 0 {
            return Err(Error::EOF);
        }

        let mut request_line = buf.trim_end().split_ascii_whitespace();
        let method = Method::parse(request_line.next().ok_or(Error::MalformedInput)?)?;
        let uri = request_line
            .next()
            .ok_or(Error::MalformedInput)?
            .to_string();
        let version = Version::parse(request_line.next().ok_or(Error::MalformedInput)?)?;
        if request_line.next().is_some() {
            return Err(Error::MalformedInput);
        }

        let mut headers = HashMap::new();
        buf.clear();
        // Use `read_line()` instead of the `lines()` iterator,
        // to prevent allocating string on every line.
        while reader.read_line(buf)? > 0 {
            let line = buf.trim_end();
            // hit empty line of \r\n
            if line.is_empty() {
                break;
            }
            let mut header = line.split(": ");
            // HTTP headers are case-insensitive.
            let key = header.next().ok_or(Error::MalformedInput)?.to_lowercase();
            let value = header.next().ok_or(Error::MalformedInput)?.to_lowercase();
            if header.next().is_some() {
                return Err(Error::MalformedInput);
            }
            headers.entry(key).or_insert(Vec::new()).push(value);
            buf.clear();
        }

        Ok(Request {
            method,
            uri,
            version,
            headers,
        })
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Response {
            version,
            status,
            reason,
            headers,
            body,
        } = self;
        write!(f, "{version} {status} {reason}\r\n")?;
        for (key, values) in headers {
            for value in values {
                write!(f, "{key}: {value}\r\n")?;
            }
        }
        write!(f, "\r\n")?;
        if let Some(body) = body {
            write!(f, "{body}\r\n")?;
        }
        Ok(())
    }
}

pub fn handle_http_request(buf: &mut String, req: &Request, web_root: &Path) -> Result<Response> {
    let mut path = web_root.join(&req.uri[1..]);
    if req.uri.ends_with('/') {
        path.push("/index.html");
    }

    let mut resp = Response {
        version: req.version,
        status: 200,
        reason: "OK".into(),
        headers: HashMap::from([("Server".into(), vec!["http-server/v0.1.0".into()])]),
        body: None,
    };

    if !path.exists() || !fs::canonicalize(&path)?.starts_with(web_root) {
        resp.status = 404;
        resp.reason = "Not Found".into();
    } else {
        fs::File::open(path)?.read_to_string(buf)?;
        resp.body = Some(mem::take(buf));
    }

    resp.headers.insert(
        "Content-Length".into(),
        vec![resp.body.as_deref().map(str::len).unwrap_or(0).to_string()],
    );

    Ok(resp)
}

#[cfg(test)]
mod test {
    use std::{
        env,
        io::{BufReader, Cursor},
    };

    use super::*;

    #[test]
    fn parse_basic_get() -> Result<()> {
        let s = "GET / HTTP/1.0\r\nHost: 127.0.0.1:8000\r\nUser-Agent: curl/8.8.0\r\nAccept: */*\r\n\r\n";
        let req = Request::parse(&mut BufReader::new(Cursor::new(s)), &mut String::new())?;
        assert_eq!(
            req,
            Request {
                method: Method::Get,
                uri: "/".into(),
                version: Version::Http1_0,
                headers: HashMap::from([
                    ("host".into(), vec!["127.0.0.1:8000".into()]),
                    ("user-agent".into(), vec!["curl/8.8.0".into()]),
                    ("accept".into(), vec!["*/*".into()]),
                ])
            }
        );
        Ok(())
    }

    #[test]
    fn serialize() -> Result<()> {
        assert_eq!(
            Response {
                version: Version::Http1_0,
                status: 200,
                reason: "OK".into(),
                headers: HashMap::from([("Connection".into(), vec!["Closed".into()])]),
                body: None
            }
            .to_string(),
            "HTTP/1.0 200 OK\r\nConnection: Closed\r\n\r\n"
        );

        Ok(())
    }

    #[test]
    fn response_file() -> Result<()> {
        let req = Request {
            method: Method::Get,
            uri: "/src/lib.rs".into(),
            version: Version::Http1_0,
            headers: HashMap::new(),
        };
        let resp = handle_http_request(&mut String::new(), &req, &env::current_dir()?)?;

        assert_eq!(resp.status, 200);
        assert!(resp.headers.contains_key("Content-Length"));
        assert!(resp.body.is_some());

        Ok(())
    }

    #[test]
    fn not_read_outside() -> Result<()> {
        let req = Request {
            method: Method::Get,
            uri: "../Cargo.toml".into(),
            version: Version::Http1_0,
            headers: HashMap::new(),
        };
        let resp = handle_http_request(&mut String::new(), &req, &env::current_dir()?.join("src"))?;

        assert_eq!(resp.status, 404);
        assert!(resp.body.is_none());

        Ok(())
    }
}
