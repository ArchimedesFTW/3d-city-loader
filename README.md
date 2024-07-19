
# 3D City Loader

Joint project with @DukeBas and @w2ptr .

## Controls

Running the pre-built executable will open a window that has two parts:

- The loader panel, which is a small floating panel that can be collapsed and moved around.

- The earth panel, which takes up the full width of the screen. It can be accessed by clicking anywhere on the panel,
  and focus can be returned to the load panael by pressing the Escape button.

The loader panel has three options:

- A "City" option, which takes the name of a city like "Netersel" or "Berlin" (has to be capitalized). Pressing the
  "LOAD" button should send a request over the network to the Overpass API, which can take quite some time for large
  cities. After receiving the data, the application will display a confirmation message and create the city;

- A "File" option, which takes an absolute or relative file path to a `.json` file on the computer. One useful trick is
  that the app will store the latest query in the file `./geocache/last.json`, so entering that file here can save a
  lot of time if you are trying to load the same city as during a previous run;

- An "Overpass" option, which takes a raw OverpassQL query. Note that the output format is still expected to be JSON,
  so `[out:json];` is required at the start of the query.

The earth panel shows a first-person view of the data that was loaded, once focus is transferred to it by clicking the
panel with the mouse. The following controls can be used:

- W, A, S, D for moving forward, left, backward, right respectively;

- Spacebar and Shift for moving up and down respectively;

- Dragging the mouse or touchpad for rotating around the camera;

- Escape for transferring the focus back to the user interface (the earth loader panel).

## Building/Running the Project

First of all, [install `cargo`](https://doc.rust-lang.org/cargo/getting-started/installation.html).

Then run the `release` (fastest) version of the project with:

```sh
cargo run --release
```

## Running on WebAssembly

Note that this does not seem to work on windows! Consider using WSL in that case.

To set up, install the target and the bindgen tool as follows:

```sh
rustup target install wasm32-unknown-unknown
cargo install wasm-server-runner # optional, for running on local server only
cargo install wasm-bindgen-cli
```

Build the executable as follows:

```sh
cargo build --target wasm32-unknown-unknown --release
```

Generate a folder for embedding in a web page as follows:

```sh
wasm-bindgen --no-typescript --target web --out-dir ./out/ --out-name "city_visualizer" ./target/wasm32-unknown-unknown/release/city_visualizer.wasm
```

Then copy the `assets` folder in its entirety to that `out` folder. The `out` folder can then be served on a web
server. Note that the performance is quite bad as WebAssembly does not have support for multithreading and is
generally going to be slower.
