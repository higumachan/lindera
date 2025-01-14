use std::env;
use std::error::Error;
use std::fs::{create_dir, rename};
use std::io::Cursor;
use std::path::Path;

use encoding::all::EUC_JP;
use encoding::{EncoderTrap, Encoding};

use flate2::read::GzDecoder;
use tar::Archive;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use lindera_core::dictionary_builder::DictionaryBuilder;
use lindera_ipadic_builder::ipadic_builder::IpadicBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = env::var_os("OUT_DIR").unwrap(); // ex) target/debug/build/<pkg>/out

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    let ipadic_ver = "2.7.0-20070801";
    let input_dir = Path::new(&out_dir).join(format!("mecab-ipadic-{}", ipadic_ver));

    if std::env::var("DOCS_RS").is_ok() {
        // Use dummy data in docs.rs.
        create_dir(&input_dir)?;

        let mut dummy_char_def = File::create(input_dir.join("char.def")).await?;
        dummy_char_def.write_all(b"DEFAULT 0 1 0\n").await?;

        let mut dummy_dict_csv = File::create(input_dir.join("dummy_dict.csv")).await?;
        dummy_dict_csv
            .write_all(
                &EUC_JP
                    .encode(
                        "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
                        EncoderTrap::Ignore,
                    )
                    .unwrap(),
            )
            .await?;

        File::create(input_dir.join("unk.def")).await?;
        let mut dummy_matrix_def = File::create(input_dir.join("matrix.def")).await?;
        dummy_matrix_def.write_all(b"0 1 0\n").await?;
    } else {
        let file_name = format!("mecab-ipadic-{}.tar.gz", ipadic_ver);

        let dest_path = Path::new(&out_dir).join(&file_name);
        if !dest_path.exists() {
            let tmp_path = Path::new(&out_dir).join(file_name + ".download");

            // Download a tarball
            let download_url =
                "https://drive.google.com/uc?export=download&id=0B4y35FiV1wh7MWVlSDBCSXZMTXM";
            let mut resp = reqwest::get(download_url).await?;

            // Save a ttarball
            let mut dest = File::create(&tmp_path).await?;
            while let Some(chunk) = resp.chunk().await? {
                dest.write_all(&chunk).await?;
            }
            rename(tmp_path, &dest_path).expect("Failed to rename temporary file");
        }

        // Decompress a tarball
        let mut tar_gz = File::open(&dest_path).await?;
        let mut buffer = Vec::new();
        tar_gz.read_to_end(&mut buffer).await?;
        let cursor = Cursor::new(buffer);
        let gzdecoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(gzdecoder);
        archive.unpack(&out_dir)?;
    }

    // Build dictionary
    let output_dir = Path::new(&out_dir).join("lindera-ipadic");

    let builder = IpadicBuilder::new();
    builder.build_dictionary(&input_dir, &output_dir)?;

    Ok(())
}
