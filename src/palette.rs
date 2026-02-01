use tint::Srgb;

/// Matches `palette` on [`colorgrad`] preset functions.
pub fn parse_palette(palette: &str) -> Vec<Srgb> {
    macro_rules! match_colorgrad {
        (palette, $($arm:ident,)*) => {
            match palette {
                $(stringify!($arm) => generate_gradient(&colorgrad::preset::$arm()),)*
                _ => {
                    println!("Unknown palette: {}", palette);
                    std::process::exit(1);
                }
            }
        };
    }

    match_colorgrad!(
        palette,
        blues,
        br_bg,
        bu_gn,
        bu_pu,
        cividis,
        cool,
        cubehelix_default,
        gn_bu,
        greens,
        greys,
        inferno,
        magma,
        or_rd,
        oranges,
        pi_yg,
        plasma,
        pr_gn,
        pu_bu,
        pu_bu_gn,
        pu_or,
        pu_rd,
        purples,
        rainbow,
        rd_bu,
        rd_gy,
        rd_pu,
        rd_yl_bu,
        rd_yl_gn,
        reds,
        sinebow,
        spectral,
        turbo,
        viridis,
        warm,
        yl_gn,
        yl_gn_bu,
        yl_or_br,
        yl_or_rd,
    )
}

fn generate_gradient(grad: &impl colorgrad::Gradient) -> Vec<Srgb> {
    let mut palette = Vec::new();
    let samples = 16;
    for x in 0..=samples {
        let rgb = grad.at(x as f32 / samples as f32);
        let [r, g, b, _] = rgb.to_rgba8();
        palette.push(Srgb::new(r, g, b, 255));
    }
    for x in (1..samples).rev() {
        let rgb = grad.at(x as f32 / samples as f32);
        let [r, g, b, _] = rgb.to_rgba8();
        palette.push(Srgb::new(r, g, b, 255));
    }
    palette
}
