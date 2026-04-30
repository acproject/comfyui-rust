const TYPE_COLORS: Record<string, string> = {
  MODEL: '#b39ddb',
  CLIP: '#fdd835',
  VAE: '#ff6e6e',
  CONDITIONING: '#ffa726',
  LATENT: '#ff9cfb',
  IMAGE: '#64b5f6',
  MASK: '#81c784',
  CONTROL_NET: '#00e5ff',
  INT: '#7986cb',
  FLOAT: '#7986cb',
  STRING: '#ce93d8',
  BOOLEAN: '#ce93d8',
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
  _default: '#4a5568',
};

function getCategoryColor(category: string): string {
  for (const [key, color] of Object.entries(CATEGORY_COLORS)) {
    if (category.toLowerCase().includes(key)) return color;
  }
  return CATEGORY_COLORS._default;
}

export { TYPE_COLORS, getTypeColor, CATEGORY_COLORS, getCategoryColor };
