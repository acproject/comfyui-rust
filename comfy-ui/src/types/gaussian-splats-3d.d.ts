declare module '@mkkellogg/gaussian-splats-3d' {
  export enum SceneFormat {
    PLY = 0,
    Splat = 1,
    KSplat = 2,
  }

  export interface ViewerOptions {
    rootElement: HTMLElement;
    cameraUp?: [number, number, number];
    initialCameraPosition?: [number, number, number];
    initialCameraLookAt?: [number, number, number];
    sharedMemoryForWorkers?: boolean;
    integerBasedSort?: boolean;
    halfPrecisionCovariancesOnGPU?: boolean;
    dynamicScene?: boolean;
    gaussianSphereColors?: Record<number, [number, number, number]>;
    antialiased?: boolean;
  }

  export interface AddSplatSceneOptions {
    splatAlphaRemovalThreshold?: number;
    showLoadingUI?: boolean;
    position?: [number, number, number];
    rotation?: [number, number, number];
    scale?: [number, number, number];
    format?: SceneFormat;
  }

  export class Viewer {
    constructor(options: ViewerOptions);
    addSplatScene(path: string, options?: AddSplatSceneOptions): Promise<void>;
    start(): void;
    dispose(): void;
    getSplatCount(): number;
  }

  export class DropInViewer {
    constructor(options?: any);
  }
}
