const TYPE_COLORS: Record<string, string> = {
  MODEL: '#b39ddb',
  CLIP: '#fdd835',
  VAE: '#ff6e6e',
  LLM: '#81d4fa',
  IMAGE: '#64b5f6',
  VIDEO: '#4fc3f7',
  AUDIO: '#4dd0e1',
  MASK: '#81c784',
  LATENT: '#ff9cfb',
  CONDITIONING: '#ffa726',
  CONTROL_NET: '#00e5ff',
  CLIP_VISION: '#7c4dff',
  STYLE_MODEL: '#aa00ff',
  UPSCALE_MODEL: '#d500f9',
  GLIGEN: '#e040fb',
  PHOTOMAKER: '#ea80fc',
  HYPERNETWORK: '#f48fb1',
  EMBEDDING: '#f8bbd9',
  NOISE: '#90a4ae',
  SAMPLER: '#78909c',
  SIGMAS: '#607d8b',
  GUIDER: '#546e7a',
  GUIDER_PARAMETERS: '#455a64',
  LORA: '#cddc39',
  LATENT_UPSCALE_MODEL: '#c0ca33',
  INT: '#7986cb',
  FLOAT: '#7986cb',
  STRING: '#ce93d8',
  BOOLEAN: '#ce93d8',
  RELAY_OPTIONS: '#cddc39',
  GUIDE_DATA: '#26c6da',
  GAUSSIAN_3D: '#00e676',
  H3_CONTEXT: '#ff7043',
  COMBO: '#90a4ae',
  '*': '#9e9e9e',
};

function getTypeColor(typeName: string): string {
  return TYPE_COLORS[typeName.toUpperCase()] || TYPE_COLORS['*'];
}

const CATEGORY_COLORS: Record<string, string> = {
  loaders: '#5b8c5a',
  conditioning: '#c78030',
  sampling: '#5a6abf',
  latent: '#7a5bbf',
  image: '#bf5b7a',
  mask: '#8b5bbf',
  '3d': '#00897b',
  custom: '#8b6bbf',
  whatdreamscost: '#1e88e5',
  h3: '#ff7043',
  _default: '#4a5568',
};

function getCategoryColor(category: string): string {
  for (const [key, color] of Object.entries(CATEGORY_COLORS)) {
    if (category.toLowerCase().includes(key)) return color;
  }
  return CATEGORY_COLORS._default;
}

function isCustomNode(classType: string): boolean {
  return classType.startsWith('Custom_');
}

export { TYPE_COLORS, getTypeColor, CATEGORY_COLORS, getCategoryColor, isCustomNode };
