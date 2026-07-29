export interface ContentAsset {
  id: string;
  unitId: string;
  publicPath: string;
}

export interface ContentUnit {
  id: string;
  kind: string;
  status?: string;
  assetIds: string[];
}

export interface ContentRelease {
  releaseId: string;
  units: Map<string, ContentUnit>;
  assets: Map<string, ContentAsset>;
}
