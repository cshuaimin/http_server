use std::{
    env, fs,
    io::{prelude::*, BufReader, BufWriter},
    mem,
    net::{TcpListener, TcpStream},
    path::Path,
    sync::Arc,
    thread,
};

use http_server::{
    error::{Error, Result},
    http::{handle_http_request, Request, Version},
    thread_pool::ThreadPool,
};

fn main() -> Result<()> {
    let mut args = env::args();
    let prog = args.next();
    let host = args.next();
    let host = host.as_deref().unwrap_or("127.0.0.1");
    if host == "-h" || host == "--help" {
        let prog = prog.unwrap();
        eprintln!("A sinple HTTP server");
        eprintln!("Usage: {prog} [host] [port] [web_root] [num_threads]");
        return Ok(());
    }

    let port = args
        .next()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8000);
    let web_root = Arc::from(fs::canonicalize(
        args.next()
            .as_ref()
            .map(Path::new)
            .unwrap_or(env::current_dir()?.as_ref()),
    )?);
    let num_threads = args
        .next()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(thread::available_parallelism()?.get());

    let listener = TcpListener::bind((host, port))?;
    println!("Listening on {}:{}", host, port);

    let thread_pool = ThreadPool::new(num_threads, handle_connection);

    for stream in listener.incoming() {
        let stream = stream?;
        thread_pool.run((stream, Arc::clone(&web_root)));
    }

    Ok(())
}

fn handle_connection(buf: &mut String, (stream, web_root): (TcpStream, Arc<Path>)) -> Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);

    loop {
        let req = match Request::parse(&mut reader, buf) {
            Ok(req) => req,
            // Close the connection on EOF.
            Err(Error::EOF) => break,
            Err(err) => return Err(err),
        };

        let mut resp = handle_http_request(buf, &req, &web_root)?;
        write!(writer, "{resp}")?;
        writer.flush()?;
        if let Some(body) = &mut resp.body {
            // Restore the per-thread buffer as it war taken in `handle_http_request()`.
            mem::swap(buf, body);
        }

        // HTTP 1.0 connections are short-lived.
        if req.version == Version::Http1_0
            || req.headers.get("connection").map(|vec| vec[0].as_str()) == Some("close")
        {
            break;
        }
    }

    Ok(())
}
