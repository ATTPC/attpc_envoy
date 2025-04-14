# attpc_envoy

attpc_envoy is a Data Acquisition Hub for the Active Target Time Projection Chamber 
written in Rust. It provides an async framework for querying the various DAQ elements 
of the AT-TPC with a clean user interface. The primary goal is to provide an error-safe
system from which to run and manage the data aqcuistion, without sacrificing 
performance.

## Download and Installation

To build attpc_envoy you need a rust compiler which can be installed using instructions
from the Rust [website](https://www.rust-lang.org).

To download attpc_envoy use the command: 

```bash
git clone https://github.com/gwm17/attpc_envoy.git
```

To build the project and run it, enter the repository and use the following command

```bash
cargo run -r
```

attpc_envoy aims to be cross platform and supports Mac, Windows, and Linux. However, 
since the AT-TPC group primarily uses MacOS for DAQ machines, attpc_envoy is only 
guaranteed to run on MacOS. Other platforms are supported as best-effort only.

## What is attpc_envoy exactly?

attpc_envoy is essentially a control panel that communicates to the distributed DAQ
services used to run the AT-TPC. There are a few key assumptions made by the envoy 
system that you should be aware of:

- attpc_envoy assumes that you are using the AT-TPC network infrastructure, including
the expected IP address assignments for DAQ machines.
- Each DAQ machine is expected to be running three programs: `getECCServer`, 
`dataRouter` or `DataExporter`, and `attpc_sentry`
- There are the typical number of DAQ machines (at time of writting, 11)
- GETDAQ configuration files are located in the expected AT-TPC directory structure

If you do not meet these requirements, or are unsure, you can checkout the 
[constants](https://attpc.github.io/attpc_envoy/attpc_envoy/envoy/constants/index.html)
docs to view some of them.

## Configuration

It is not recommended to create a configuration by hand. Instead, it is best to use the
UI to set configuration parameters and then use the File->Save, and File->Load menus to
handle configuration saving and loading. However, we will outline the format here for
clarity.

attpc_envoy uses YAML to define its configuration. Below is an example configuration 
file:

```yaml
experiment: Exp
run_number: 0
description: Write here
fields:
  B-Field (T): ''
  Beam: ''
  E-Drift (V): ''
  E-Trans (V): ''
  Energy (MeV/U): ''
  GET Freq. (MHz): ''
  Pressure (Torr): ''
  Target Gas: ''
  V_Cathode (kV): ''
  V_MM (V): ''
  V_THGEM (V): ''
```

`experiment` is the experiment identifier, and should match the identifier used for the
GET configuration files. `run_number` is the current run number. `description` is a 
brief discription of the run. `fields` is an extensible list of run logged info. All 
`fields` values are stored as strings even if they are more naturally a numeric type to
avoid boxing.

## Documentation

The documentation for attpc_envoy can be found [here](https://attpc.github.io/attpc_envoy)
