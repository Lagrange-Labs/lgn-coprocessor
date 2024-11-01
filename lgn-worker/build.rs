use miette::IntoDiagnostic;

fn main() -> miette::Result<()>
{
    let file_descriptors = protox::compile(
        ["proto/lagrange.proto"],
        ["../lagrange-protobuf/"],
    )?;

    tonic_build::configure()
        .build_server(true)
        .compile_fds(file_descriptors)
        .into_diagnostic()?;
    Ok(())
}
