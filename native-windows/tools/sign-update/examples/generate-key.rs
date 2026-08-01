use std::{env, fs, path::PathBuf, process::ExitCode};

use minisign::KeyPair;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), &'static str> {
    let mut args = env::args_os().skip(1);
    let private_path = args
        .next()
        .map(PathBuf::from)
        .ok_or("missing private key path")?;
    let public_path = args
        .next()
        .map(PathBuf::from)
        .ok_or("missing public key path")?;
    if args.next().is_some() {
        return Err("unexpected arguments");
    }

    let key_pair =
        KeyPair::generate_unencrypted_keypair().map_err(|_| "unable to generate signing key")?;
    let private_key = key_pair
        .sk
        .to_box(Some("Codex Gauge Windows updater signing key"))
        .map_err(|_| "unable to encode private key")?;
    let public_key = key_pair
        .pk
        .to_box()
        .map_err(|_| "unable to encode public key")?;

    fs::write(private_path, private_key.into_string())
        .map_err(|_| "unable to write private key")?;
    fs::write(public_path, public_key.into_string()).map_err(|_| "unable to write public key")?;
    Ok(())
}
