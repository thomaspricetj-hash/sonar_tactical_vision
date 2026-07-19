Close‑Range Sonar Perception System

High‑precision near‑field sensing, reflex actions, hazard mapping, cross‑section analysis, and multi‑sensor fusion for robots and autonomous vehicles.

This project implements a full close‑range perception stack designed for robots, drones, and autonomous vehicles operating in tight spaces. It provides:



Real‑time sonar processing



Multi‑layer heatmaps (predictive, temporal, gradient, flow)



Cross‑Section Mapping Engine (NEW)



Semantic classification



Reflex engine



Hazard memory



Multi‑sensor fusion (sonar + vision + LiDAR + radar)



Steering‑aware tactical events



Deterministic runtime loop



Modular architecture



The system is built in Rust and structured like a lightweight robotics perception engine.



Features

Sonar Engine

Converts raw sonar readings into normalized multi‑layer heatmaps:



Predictive forward‑projection



Temporal accumulation



Gradient edge detection



Flow‑based motion vectors



Deterministic fusion into a composite heatmap



Noise‑robust and deterministic.



Cross‑Section Mapping Engine (NEW)

Transforms fused heatmaps into high‑precision spatial, temporal, and hazard‑aware slices:



Spatial Slices

Front / Back



Left / Right



Quadrants (Q1–Q4)



Radial rings (inner / mid / outer)



Motion‑Vector Drift

Average dx/dy flow



Detects incoming hazards



Reveals lateral drift and environmental motion



Temporal Stability

Frame‑to‑frame consistency



Detects flicker, noise, sudden changes



Hazard‑Aware Slices

Hazard front/back/left/right



Hazard quadrants



Hazard radial rings



Fused Precision Score

A deterministic metric combining entropy, volatility, drift, stability, and hazard weighting.



This engine dramatically improves steering accuracy, hazard prediction, and reflex reliability.



Tactical Event System

Generates reflex‑style events such as:



CollisionImminent



PredictiveCollision



EdgeDetected



MotionFlowHazard



AvoidanceSteer



TransparentObject



TemporalHazard



Each event includes:



Severity scoring



Criticality detection



Optional steering direction



Hazard Map

Persistent near‑field hazard memory:



Reinforcement + decay



Semantic label storage



Reflex history



Spatial hazard slicing (via cross‑section engine)



Stable context for reflex decisions



Semantic Layer

Classifies sonar patterns into:



HighRiskZone



PersistentObstacle



DirectionalHazard



SoftContact



TransparentObject



TemporalHazard



Reflex Pipeline (Fusion‑Aware)

Blends:



Sonar tactical events



Semantic meaning



Cross‑section slices



Hazard slices



Multi‑sensor fusion hazard



Fused precision score



Produces:



EmergencyStop



SlowDown



SteerAway



MarkHazard



None



Multi‑Sensor Fusion

Combines:



Sonar hazard map



Vision confidence



LiDAR confidence



Radar confidence



Outputs:



Fused hazard level



Fused confidence



Recommended reflex



Runtime

Deterministic tick loop:



Time‑based routing



Multi‑layer heatmap updates



Cross‑section mapping



Hazard map reinforcement



Event generation



Semantic classification



Fusion integration



Reflex output



Supports external sensor injection.



Installation

Prerequisites

Rust (stable toolchain)



Cargo



A sonar device driver implementing the SonarDevice trait



Optional: camera/LiDAR/radar adapters



Clone the repository

bash

git clone https://github.com/yourname/close-range-sonar.git

cd close-range-sonar

Build

bash

cargo build --release

Run tests

bash

cargo test

Project Structure

Code

src/

&#x20;├── device/                 # Sonar device trait + drivers

&#x20;├── engine/                 # Sonar engine + multi-layer heatmaps

&#x20;├── heatmap.rs              # Cross-section mapping + fusion layers (NEW)

&#x20;├── events.rs               # TacticalEvent system (steering-aware)

&#x20;├── hazard\_map.rs           # Hazard memory + reinforcement

&#x20;├── semantic\_layer.rs       # Semantic classification

&#x20;├── reflex\_pipeline.rs      # Fusion-aware reflex engine

&#x20;├── sonar\_fusion.rs         # Multi-sensor fusion module

&#x20;├── sonar\_router.rs         # Routing + event generation + cross-sections

&#x20;├── sonar\_runtime.rs        # Deterministic runtime loop

&#x20;└── cli/                    # Optional CLI tools

How to Use

1\. Implement a Sonar Device

rust

impl SonarDevice for MySonar {

&#x20;   fn read(\&mut self) -> SonarReading {

&#x20;       // return distance + confidence

&#x20;   }

}

2\. Create a Router

rust

let device = MySonar::new();

let router = SonarRouter::new(device);

3\. Create the Runtime

rust

let mut runtime = SonarRuntime::new(router, 30.0); // 30 FPS

4\. Inject External Sensors (optional)

rust

runtime.update\_external\_sensors(FusionSensors {

&#x20;   vision\_obstacle\_confidence: 0.3,

&#x20;   lidar\_obstacle\_confidence: 0.6,

&#x20;   radar\_obstacle\_confidence: 0.1,

&#x20;   nearest\_distance\_m: Some(0.8),

});

5\. Run Frames

rust

runtime.run\_frames(100);

6\. Read Outputs

rust

if let Some(output) = runtime.latest() {

&#x20;   println!("Sonar output: {:?}", output);

}



if let Some(fusion) = runtime.latest\_fusion() {

&#x20;   println!("Fusion state: {:?}", fusion);

}

7\. Reflex Decision

rust

let reflex = runtime

&#x20;   .router()

&#x20;   .reflex\_pipeline()

&#x20;   .handle\_with\_fusion(

&#x20;       \&output.events,

&#x20;       \&output.semantic,

&#x20;       runtime.latest\_fusion(),

&#x20;   );



println!("Reflex: {:?}", reflex);

Example Reflex Output

Code

Reflex: SteerAway { angle\_deg: -32.5 }

Meaning:



Hazard detected



Steering vector computed



Robot should turn left \~32°



Example Fusion Output

Code

FusionState {

&#x20;   fused\_hazard\_level: 0.72,

&#x20;   fused\_confidence: 0.88,

&#x20;   recommended\_reflex: SlowDown

}

Why This System Matters

Close‑range perception is the hardest part of robotics:



Cameras fail at <30cm



LiDAR has blind spots



Radar lacks resolution



Sonar excels at the last meter



This system gives robots reflexes, not just sensors.



It behaves more like a biological organism:



Detect



Interpret



React



Remember



Adapt



The new Cross‑Section Mapping Engine dramatically improves:



steering accuracy



hazard prediction



temporal stability



motion‑aware reflexes



multi‑layer fusion precision

