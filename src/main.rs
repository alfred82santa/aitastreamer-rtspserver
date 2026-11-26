// This example demonstrates how to set up a rtsp server using GStreamer.
// For this, the example parses an arbitrary pipeline in launch syntax
// from the cli and provides this pipeline's output as stream, served
// using GStreamers rtsp server.

use std::env;

use crate::cli::Cli;
use crate::mediafactories::MediaFactoryManager;
use anyhow::Error;
use clap::Parser;
use derive_more::{Display, Error};
use gstreamer_rtsp_server::prelude::*;
use gstreamer_rtsp_server::{glib, gst, RTSPServer};

pub mod cli;
pub mod mediafactories;

#[derive(Debug, Display, Error)]
#[display("Could not get mount points")]
struct NoMountPoints;

#[derive(Debug, Display, Error)]
#[display("Usage: LAUNCH_LINE")]
struct UsageError;

fn main_loop(config: String) -> Result<(), Error> {
    let main_loop = glib::MainLoop::new(None, false);
    let server = RTSPServer::new();
    // Much like HTTP servers, RTSP servers have multiple endpoints that
    // provide different streams. Here, we ask our server to give
    // us a reference to his list of endpoints, so we can add our
    // test endpoint, providing the pipeline from the cli.
    let mounts: gstreamer_rtsp_server::RTSPMountPoints =
        server.mount_points().ok_or(NoMountPoints)?;

    let _manager = MediaFactoryManager::new_from_yaml_str(mounts, &config);

    // Attach the server to our main context.
    // A main context is the thing where other stuff is registering itself for its
    // events (e.g. sockets, GStreamer bus, ...) and the main loop is something that
    // polls the main context for its events and dispatches them to whoever is
    // interested in them. In this example, we only do have one, so we can
    // leave the context parameter empty, it will automatically select
    // the default one.
    let id = server.attach(None)?;

    println!("Stream ready at rtsp://{}:{}/test", server.address().unwrap(), server.service());

    // Start the mainloop. From this point on, the server will start to serve
    // our quality content to connecting clients.
    main_loop.run();

    id.remove();

    Ok(())
}

fn server_main(config: String) -> Result<(), Error> {
    gst::init()?;
    main_loop(config)
}

fn main() {
    let args = Cli::parse();

    server_main(args.get_config_str()).expect("msg")
}
