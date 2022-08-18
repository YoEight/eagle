pub fn generate() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = "src/google/generated";
    let files = ["protos/googleapis/google/monitoring/v3/metric_service.proto"];

    std::fs::create_dir_all(out_dir)?;

    tonic_build::configure()
        .build_server(false)
        .out_dir(out_dir)
        .compile(&files, &["protos/googleapis"])?;

    let mut gen_dir = std::fs::read_dir(out_dir)?;

    while let Some(file) = gen_dir.next().transpose()? {
        let filename_string = file.file_name().into_string().expect("valid utf-8 string");
        let len = filename_string.chars().count();

        let (name, _) = filename_string.split_at(len - 3);
        let new_name = name.replace(".", "_");
        let new_name = format!("{}.rs", new_name);
        let new_file = file.path().parent().unwrap().join(new_name);

        std::fs::rename(file.path(), new_file)?;
    }

    Ok(())
}

fn main() {
    generate().unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
}
