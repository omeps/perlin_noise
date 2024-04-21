fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(perlin_noise::run());
    Ok(())
}
