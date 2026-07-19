use crate::heatmap::HeatLayer;

/// A standalone compression trait for heatmaps.
/// Mirrors your SyntheticMind compression API but remains independent.
/// Later, you can plug in your v8‑Ultra reversible 4‑byte collapse engine.
pub trait HeatmapCompressor {
    fn compress(&self, layer: &HeatLayer) -> Vec<u8>;
    fn decompress(&self, data: &[u8]) -> HeatLayer;
}

/// Baseline compressor using delta‑RLE + quantization.
/// Lightweight, stable, and easily replaceable with your v8‑Ultra engine.
pub struct BaselineHeatmapCompressor;

impl BaselineHeatmapCompressor {
    pub fn new() -> Self {
        Self {}
    }
}

impl HeatmapCompressor for BaselineHeatmapCompressor {
    fn compress(&self, layer: &HeatLayer) -> Vec<u8> {
        let mut out = Vec::new();

        // Write width + height (8 bytes)
        out.extend_from_slice(&(layer.width as u32).to_le_bytes());
        out.extend_from_slice(&(layer.height as u32).to_le_bytes());

        // Quantize to 0–255 (no mut needed)
        let quantized: Vec<u8> = layer
            .cells
            .iter()
            .map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8)
            .collect();

        // Delta‑RLE compression
        let mut i = 0;
        let len = quantized.len();

        while i < len {
            let value = quantized[i];
            let mut run = 1;

            // Count run length
            while i + run < len && quantized[i + run] == value && run < 255 {
                run += 1;
            }

            // Write (value, run)
            out.push(value);
            out.push(run as u8);

            i += run;
        }

        out
    }

    fn decompress(&self, data: &[u8]) -> HeatLayer {
        // Must have at least width + height
        if data.len() < 8 {
            return HeatLayer {
                width: 0,
                height: 0,
                cells: Vec::new(),
            };
        }

        // Read width + height
        let w = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let h = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;

        let mut cells = Vec::new();
        let mut i = 8;

        // Delta‑RLE decode
        while i + 1 < data.len() {
            let value = data[i];
            let run = data[i + 1] as usize;

            let decoded = (value as f32) / 255.0;

            // Push run-length values
            cells.extend(std::iter::repeat(decoded).take(run));

            i += 2;
        }

        HeatLayer {
            width: w,
            height: h,
            cells,
        }
    }
}
