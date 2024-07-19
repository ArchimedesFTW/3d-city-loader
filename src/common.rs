
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
#[cfg(not(target_arch = "wasm32"))]
use bevy::tasks::{futures_lite::future, Task};

use bevy_mod_reqwest::reqwest::StatusCode;

use std::fmt::{Display, Formatter};
use std::future::Future;
use std::path::Path;

/// The (external) format for geographic input data.
/// 
/// The internal in-memory format is described [`GeoData`].
/// 
/// [`GeoData`]: ../data/geography/struct.GeoData.html
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataFormat {
    OsmJson,
    GeoJson,
}

impl Display for DataFormat {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            DataFormat::OsmJson => write!(f, "osm json"),
            DataFormat::GeoJson => write!(f, "geojson"),
        }
    }
}

/// The error type used throughout this crate.
#[derive(Debug)]
pub enum AppError {
    /// An immediate error in the query input string.
    InputSyntax {
        message: String,
    },
    /// An input/output error.
    Io {
        url: Option<String>,
        status: Option<StatusCode>,
        message: String,
    },
    /// An error in the syntax of external data, or it being in an
    /// incorrect/unrecognized format.
    DataSyntax {
        format: DataFormat,
        line: Option<usize>,
        character: Option<usize>,
        message: String,
    },
    /// The request/query was successful, but no data was found or data is
    /// missing that is supposed to be there.
    MissingData {
        message: String,
    },
}

impl AppError {
    /// Converts a `serde_json` error to the internal error type.
    pub fn from_json_error(value: serde_json::Error, format: DataFormat) -> Self {
        AppError::DataSyntax {
            format,
            line: Some(value.line()),
            character: Some(value.column()),
            message: "Syntax error in JSON".to_owned(),
        }
    }

    pub fn from_io_error(value: std::io::Error, path: &Path) -> Self {
        AppError::Io {
            url: path.to_str().map(|x| format!("file://{}", x)),
            status: None,
            message: value.to_string(),
        }
    }
}

impl From<bevy_mod_reqwest::reqwest::Error> for AppError {
    fn from(value: bevy_mod_reqwest::reqwest::Error) -> Self {
        let url = value.url().map(|url| url.to_string());
        let status = value.status();
        AppError::Io {
            url,
            message: value.without_url().to_string(),
            status,
        }
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            AppError::InputSyntax { message } => {
                write!(f, "{}", message)?;
            },
            AppError::Io { url, status, message } => {
                if let Some(status) = status {
                    write!(f, "status {} ", status)?;
                }
                write!(f, "{}", message)?;
                if let Some(url) = url {
                    write!(f, " from url {}", url)?;
                }
            },
            AppError::DataSyntax { format, line, character, message } => {
                write!(f, "{}", message)?;
                if line.is_some() || character.is_some() {
                    write!(f, " at")?;
                }
                if let Some(line) = line {
                    write!(f, " line {}", line)?;
                }
                if let Some(character) = character {
                    write!(f, " char {}", character)?;
                }
                write!(f, " which should be in valid {} format", format)?;
            },
            AppError::MissingData { message } => {
                write!(f, "missing data! {}", message)?;
            },
        }
        Ok(())
    }
}

/// An event to display a status message to the user.
#[derive(Event)]
pub enum StatusEvent {
    Error(AppError),
    Update(String),
}

#[derive(Component)]
pub struct AsyncComputation<T>
where
    T: Send + Sync + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    pub task: Task<T>,
    #[cfg(target_arch = "wasm32")]
    pub receiver: crossbeam_channel::Receiver<T>,
}

/// Spawns an asynchronous task in a different thread, and adds it as an
/// entity+component to the world.
/// 
/// This uses the `AsyncComputeTaskPool` to spawn the task.
pub fn spawn_compute_task<T>(
    commands: &mut Commands,
    computation: impl Future<Output = T> + Send + 'static,
)
where
    T: Send + Sync + 'static,
{
    let task_pool = AsyncComputeTaskPool::get();

    #[cfg(not(target_arch = "wasm32"))]
    {
        let task = task_pool.spawn(computation);
        // by adding this as an entity, we can poll for this event in
        // a system, which is where the produced data is then used
        commands.spawn(AsyncComputation { task });
    }

    #[cfg(target_arch = "wasm32")]
    {
        let (sender, receiver) = crossbeam_channel::bounded(1);
        task_pool.spawn(async move {
            let _ = sender.send(computation.await);
        }).detach();
        commands.spawn(AsyncComputation { receiver });
    }
}

/// Polls all async compute tasks given in `query` and calls `callback` on them
/// if they gave back a result.
pub fn handle_compute_tasks<T>(
    commands: &mut Commands,
    mut query: Query<(Entity, &mut AsyncComputation<T>)>,
    mut callback: impl FnMut(&mut Commands, T),
)
where
    T: Send + Sync + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    future::block_on(async move {
        for (id, mut computation) in &mut query {
            match future::poll_once(&mut computation.task).await {
                Some(result) => {
                    callback(commands, result);
                    commands.entity(id).remove::<AsyncComputation<T>>();
                },
                None => {},
            }
        }
    });

    #[cfg(target_arch = "wasm32")]
    for (id, computation) in &mut query {
        match computation.receiver.try_recv() {
            Ok(result) => {
                callback(commands, result);
                commands.entity(id).remove::<AsyncComputation<T>>();
            },
            Err(_) => {}, // computation does not have a result yet
        }
    }
}
