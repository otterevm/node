use std::{net::SocketAddr, path::PathBuf};

use axum::{
    Extension, Router,
    body::Body,
    http::{Response, StatusCode, header},
    routing::get,
};
use commonware_runtime::{Handle, Metrics as _, Spawner as _, tokio::Context};
use eyre::WrapErr as _;
use tokio::net::TcpListener;

/// Installs a metrics server so that commonware can publish its metrics.
///
/// This is lifted straight from [`commonware_runtime::tokio::telemetry::init`],
/// because it also wants to install a tracing subscriber, which clashes with
/// reth ethereum cli doing the same thing.
pub fn install(
    context: Context,
    listen_addr: SocketAddr,
    pprof_dump_dir: PathBuf,
) -> Handle<eyre::Result<()>> {
    context.spawn(move |context| async move {
        // Create a tokio listener for the metrics server.
        //
        // We explicitly avoid using a runtime `Listener` because
        // it will track bandwidth used for metrics and apply a policy
        // for read/write timeouts fit for a p2p network.
        let listener = TcpListener::bind(listen_addr)
            .await
            .wrap_err("failed to bind provided address")?;

        // Create a router for the metrics server
        let app = Router::new()
            .route(
                "/metrics",
                get(|Extension(ctx): Extension<Context>| async move {
                    Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "text/plain; version=0.0.4")
                        .body(Body::from(ctx.encode()))
                        .expect("Failed to create response")
                }),
            )
            .route(
                "/debug/pprof/heap",
                get({
                    let pprof_dump_dir = pprof_dump_dir.clone();
                    move || {
                        let pprof_dump_dir = pprof_dump_dir.clone();
                        async move { handle_pprof_heap(&pprof_dump_dir) }
                    }
                }),
            )
            .layer(Extension(context));

        // Serve the metrics over HTTP.
        //
        // `serve` will spawn its own tasks using `tokio::spawn` (and there is no way to specify
        // it to do otherwise). These tasks will not be tracked like metrics spawned using `Spawner`.
        axum::serve(listener, app.into_make_service())
            .await
            .map_err(Into::into)
    })
}

#[cfg(all(feature = "jemalloc-prof", unix))]
fn handle_pprof_heap(pprof_dump_dir: &PathBuf) -> Response<Body> {
    use axum::http::header::CONTENT_ENCODING;

    match jemalloc_pprof::PROF_CTL.as_ref() {
        Some(prof_ctl) => match prof_ctl.try_lock() {
            Ok(_) => match jemalloc_pprof_dump(pprof_dump_dir) {
                Ok(pprof) => {
                    let mut response = Response::new(Body::from(pprof));
                    response.headers_mut().insert(
                        header::CONTENT_TYPE,
                        "application/octet-stream".parse().unwrap(),
                    );
                    response
                        .headers_mut()
                        .insert(CONTENT_ENCODING, "gzip".parse().unwrap());
                    response
                }
                Err(err) => {
                    let mut response =
                        Response::new(Body::from(format!("Failed to dump pprof: {err}")));
                    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    response
                }
            },
            Err(_) => {
                let mut response = Response::new(Body::from(
                    "Profile dump already in progress. Try again later.",
                ));
                *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
                response
            }
        },
        None => {
            let mut response = Response::new(Body::from(
                "jemalloc profiling not enabled. \
                 Set MALLOC_CONF=prof:true or rebuild with jemalloc-prof feature.",
            ));
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            response
        }
    }
}

/// Equivalent to [`jemalloc_pprof::JemallocProfCtl::dump`], but accepts a directory that the
/// temporary pprof file will be written to. The file is deleted when the function exits.
#[cfg(all(feature = "jemalloc-prof", unix))]
fn jemalloc_pprof_dump(pprof_dump_dir: &PathBuf) -> eyre::Result<Vec<u8>> {
    use std::{ffi::CString, io::BufReader};

    use mappings::MAPPINGS;
    use pprof_util::parse_jeheap;
    use tempfile::NamedTempFile;

    std::fs::create_dir_all(pprof_dump_dir)?;
    let f = NamedTempFile::new_in(pprof_dump_dir)?;
    let path = CString::new(f.path().as_os_str().as_encoded_bytes()).unwrap();

    // SAFETY: "prof.dump" is documented as being writable and taking a C string as input:
    // http://jemalloc.net/jemalloc.3.html#prof.dump
    unsafe { tikv_jemalloc_ctl::raw::write(b"prof.dump\0", path.as_ptr()) }?;

    let dump_reader = BufReader::new(f);
    let profile =
        parse_jeheap(dump_reader, MAPPINGS.as_deref()).map_err(|err| eyre::eyre!(Box::new(err)))?;
    let pprof = profile.to_pprof(("inuse_space", "bytes"), ("space", "bytes"), None);

    Ok(pprof)
}

#[cfg(not(all(feature = "jemalloc-prof", unix)))]
fn handle_pprof_heap(_pprof_dump_dir: &PathBuf) -> Response<Body> {
    let mut response = Response::new(Body::from(
        "jemalloc pprof support not compiled. Rebuild with the jemalloc-prof feature.",
    ));
    *response.status_mut() = StatusCode::NOT_IMPLEMENTED;
    response
}
