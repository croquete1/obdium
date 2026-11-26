//! Offline-only catalog inspector; never opens an adapter or sends a request.
#[path = "../src/reference_data.rs"]
mod reference_data;

use reference_data::{decode_signal, translation_preview, BitOrder, ReferenceCatalogs};
use serde::Serialize;
use std::error::Error;

fn print_json(value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.into_string().map_err(|_| "arguments must be valid UTF-8"))
        .collect::<Result<_, _>>()?;
    match args.as_slice() {
        [kind, key] if kind == "obd" => {
            print_json(&ReferenceCatalogs::bundled()?.obd(key))?;
        }
        [kind, key] if kind == "dtc" => {
            print_json(&ReferenceCatalogs::bundled()?.dtc(key))?;
        }
        [kind, key] if kind == "nrc" => {
            let code = u8::from_str_radix(key, 16)?;
            print_json(&ReferenceCatalogs::bundled()?.nrc(code))?;
        }
        [kind, locale, key] if kind == "translate" => {
            print_json(&translation_preview(locale, key)?)?;
        }
        [kind, bytes, start, width, endian, sign, factor, offset] if kind == "decode" => {
            if bytes.len() > 128
                || !bytes.len().is_multiple_of(2)
                || !bytes.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err("expected at most 64 bytes of contiguous hexadecimal data".into());
            }
            let payload = (0..bytes.len())
                .step_by(2)
                .map(|index| u8::from_str_radix(&bytes[index..index + 2], 16))
                .collect::<Result<Vec<_>, _>>()?;
            let order = match endian.as_str() {
                "intel" => BitOrder::Intel,
                "motorola" => BitOrder::Motorola,
                _ => return Err("expected intel or motorola".into()),
            };
            let signed = match sign.as_str() {
                "signed" => true,
                "unsigned" => false,
                _ => return Err("expected signed or unsigned".into()),
            };
            print_json(&decode_signal(
                &payload,
                start.parse()?,
                width.parse()?,
                order,
                signed,
                factor.parse()?,
                offset.parse()?,
            )?)?;
        }
        _ => {
            return Err("usage: catalog_lookup obd|dtc|nrc KEY; translate LOCALE KEY; or decode HEX START WIDTH intel|motorola signed|unsigned FACTOR OFFSET".into());
        }
    }
    Ok(())
}
