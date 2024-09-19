# attpc_envoy

attpc_envoy is a Data Acquisition Hub for the Active Target Time Projection Chamber written in Rust. It provides an async framework for querying the various DAQ elements of the AT-TPC with a clean user interface. The primary goal is to provide an error-safe system from which to run and manage the data aqcuistion, without sacrificing performance.

## Download and Installation

To download attpc_envoy simply run `git clone https://github.com/gwm17/attpc_envoy.git`. With the project downloaded, the build process is relatively simple as well. attpc_envoy is written using Rust; if you have never used Rust before you will need to download the Rust toolchain, which is provided by the Rust team [here](https://www.rust-lang.org). With the toolchain installed, navigate to the attpc_envoy repository, and from the top level repository directory run the command `cargo run --release` to build and run attpc_envoy.

### Platform Support

At the time of writting, the AT-TPC DAQ is mostly run through Apple Mac systems (MacMinis and iMacs). As such this has been the primary target during development. However, attpc_envoy has aimed to be cross-platform, to stay ahead of any potential changes to the DAQ system. attpc_envoy has been tested and successfully builds on Windows 11 and Ubuntu 22.04. For MacOS, it has been tested primarily on MacOS Ventura (13).

## Configuration

attpc_envoy requires several pieces of information to run effectively. Here we'll outline the big ones:

- Experiment Name: this is a unqiue identifier for this experiment. This name should match the name used to identify the ECC configuration files given to the CoBo/Mutant ECC servers.
- Description: Currently unused. Potentially used in a automatic experiment log feature in the future.
- Run Number: The number associated with the current data-taking run. This number *must* be unique for each run.

Configurations can be saved using the File->Save menu. Configurations can then be loaded using File->Open. Configurations are serialized to YAML files using the [serde](https://serde.rs) library.

## About

### Async Envoys

At the heart of attpc_envoy is the Envoy task object. Upon starting the attpc_envoy application, a new [tokio](https://tokio.rs) async runtime is created and then handed off to the user interface (which is made using the awesome [egui](https://www.egui.rs) library). When the user clicks the Connect button in the UI the async runtime spawns a bunch of Envoy tasks. The Envoys are more-or-less communication channels which query or command the ECC server and Data Router systems of the AT-TPC data acquisition. Communication is managed through the Embassy structure, which provides a bridge between the synchronous UI and the asynchronous Envoys. The reason for async is clear: most of the time the Envoys don't do anything! They're either waiting for a commmand from the UI or they're waiting until a timer goes off to check the status of the server. Creating a OS thread for each of these objects would be incredibly heavy. Instead, the tokio runtime manages these tasks efficiently for us, reducing the cost of having so much of the program sleeping. Under the hood, each Envoy is really just an HTTP request machine (using the [reqwest](https://crates.io/crates/reqwest) library), either posting or getting data as specified by it's task type.

### User Interface

The user interface provides the functionality to command the ECC servers to configure themeselves using the Progress/Regress buttons. Each ECC Envoy has it's own Progress/Regress buttons as well as a status. Additionally there is a system Progress/Regress set of buttons as well as a system status. The system progress/regress can be used when the *entire* ECC Envoy system is at the same status. That is, when every envoy is at the same point in the configuration process, you can progress the system as a whole rather than individually pressing each button for each envoy. However, if one of the envoys is at a different status, you will not be able to modify the system as a whole (the system status should say Inconsistent in this case). In general, where possible it is best to use the system Progress/Regress and only use individual options when the system options are not available.

Once the project is stable, a more comprehensive description of the user interface will be provided.

### Logging

A directory called `logs` is created from wherever you run attpc_envoy, and daily rotating file logs are generated and written to this directory. If an error occurs, it will be written to file as well as printed on the terminal that launched the envoy. Note that you should periodically check and clean the log directory as it does not do so by itself.
