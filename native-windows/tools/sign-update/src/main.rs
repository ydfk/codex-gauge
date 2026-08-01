use std::{
    env,
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
    process::ExitCode,
};

use minisign::SecretKeyBox;

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
    let input = args.next().map(PathBuf::from).ok_or("missing input path")?;
    let output = args
        .next()
        .map(PathBuf::from)
        .ok_or("missing signature path")?;
    if args.next().is_some() {
        return Err("unexpected arguments");
    }

    let encoded_key = env::var("NATIVE_WINDOWS_SIGNING_PRIVATE_KEY")
        .map_err(|_| "signing key is not configured")?;
    let password = env::var("NATIVE_WINDOWS_SIGNING_PRIVATE_KEY_PASSWORD")
        .ok()
        .filter(|value| !value.is_empty());

    // 私钥只在当前进程内解密，不写入磁盘或输出到日志。
    let key_box = SecretKeyBox::from_string(&encoded_key).map_err(|_| "invalid signing key")?;
    let key = key_box
        .into_secret_key(password)
        .map_err(|_| "unable to unlock signing key")?;
    let file = File::open(input).map_err(|_| "unable to read update package")?;
    let signature = minisign::sign(None, &key, BufReader::new(file), None, None)
        .map_err(|_| "unable to sign update package")?;
    fs::write(output, signature.into_string()).map_err(|_| "unable to write signature")?;
    Ok(())
}
