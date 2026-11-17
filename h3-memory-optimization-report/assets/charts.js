(function() {
  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--accent').trim();
  var accent2 = style.getPropertyValue('--accent2').trim();
  var ink = style.getPropertyValue('--ink').trim();
  var muted = style.getPropertyValue('--muted').trim();
  var rule = style.getPropertyValue('--rule').trim();
  var bg2 = style.getPropertyValue('--bg2').trim();

  // --- Chart 1: GPU Memory Allocation by Component ---
  var chart1 = echarts.init(document.getElementById('chart-memory-alloc'), null, { renderer: 'svg' });
  chart1.setOption({
    animation: false,
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' }, appendToBody: true },
    legend: { top: 0, textStyle: { color: muted, fontSize: 12 } },
    grid: { left: '3%', right: '4%', bottom: '3%', top: 40, containLabel: true },
    xAxis: {
      type: 'category',
      data: ['GPU 0\n(Transformer)', 'GPU 1\n(Language Model)', 'GPU 2\n(VAE)', 'System RAM\n(CPU offload)'],
      axisLabel: { color: muted, fontSize: 11 },
      axisLine: { lineStyle: { color: rule } }
    },
    yAxis: {
      type: 'value',
      name: '显存 / 内存 (GB)',
      nameTextStyle: { color: muted, fontSize: 11 },
      axisLabel: { color: muted, fontSize: 11 },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } }
    },
    series: [
      {
        name: '模型 Shell (常驻)',
        type: 'bar',
        stack: 'total',
        data: [0.5, 0.8, 0.3, 0],
        itemStyle: { color: accent2 },
        barWidth: '45%'
      },
      {
        name: '逐块加载 (Block offload)',
        type: 'bar',
        stack: 'total',
        data: [0.84, 0.84, 0, 60.3],
        itemStyle: { color: accent }
      },
      {
        name: '激活值 (Activations)',
        type: 'bar',
        stack: 'total',
        data: [24.0, 29.3, 7.2, 27.0],
        itemStyle: { color: accent + '66' }
      }
    ]
  });
  window.addEventListener('resize', function() { chart1.resize(); });

  // --- Chart 2: Run Timing Comparison ---
  var chart2 = echarts.init(document.getElementById('chart-timing'), null, { renderer: 'svg' });
  chart2.setOption({
    animation: false,
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' }, appendToBody: true },
    legend: { top: 0, textStyle: { color: muted, fontSize: 12 } },
    grid: { left: '3%', right: '4%', bottom: '3%', top: 40, containLabel: true },
    xAxis: {
      type: 'category',
      data: ['运行 1\n(0 常驻块)', '运行 2\n(8 常驻块)'],
      axisLabel: { color: muted, fontSize: 12 },
      axisLine: { lineStyle: { color: rule } }
    },
    yAxis: {
      type: 'value',
      name: '耗时 (分钟)',
      nameTextStyle: { color: muted, fontSize: 11 },
      axisLabel: { color: muted, fontSize: 11 },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } }
    },
    series: [
      {
        name: '模型加载',
        type: 'bar',
        stack: 'total',
        data: [8, 8],
        itemStyle: { color: accent2 },
        barWidth: '40%',
        label: { show: true, position: 'inside', color: '#fff', fontSize: 11 }
      },
      {
        name: '文本编码',
        type: 'bar',
        stack: 'total',
        data: [12, 12],
        itemStyle: { color: accent2 + '99' },
        label: { show: true, position: 'inside', color: '#fff', fontSize: 11 }
      },
      {
        name: '扩散 (30 steps)',
        type: 'bar',
        stack: 'total',
        data: [148, 143],
        itemStyle: { color: accent },
        label: { show: true, position: 'inside', color: '#fff', fontSize: 11 }
      },
      {
        name: 'VAE 解码',
        type: 'bar',
        stack: 'total',
        data: [7, 7],
        itemStyle: { color: accent + '66' },
        label: { show: true, position: 'inside', color: '#fff', fontSize: 11 }
      },
      {
        name: '视频封装',
        type: 'bar',
        stack: 'total',
        data: [4, 4],
        itemStyle: { color: muted + '88' },
        label: { show: true, position: 'inside', color: '#fff', fontSize: 11 }
      }
    ]
  });
  window.addEventListener('resize', function() { chart2.resize(); });

  // --- Chart 3: GPU Memory Timeline During Run ---
  var chart3 = echarts.init(document.getElementById('chart-memory-timeline'), null, { renderer: 'svg' });
  chart3.setOption({
    animation: false,
    tooltip: { trigger: 'axis', appendToBody: true },
    legend: { top: 0, textStyle: { color: muted, fontSize: 12 } },
    grid: { left: '3%', right: '4%', bottom: '3%', top: 40, containLabel: true },
    xAxis: {
      type: 'category',
      data: ['加载', '文本编码', '扩散 step 5', '扩散 step 15', '扩散 step 25', '扩散 step 30', 'VAE 解码'],
      axisLabel: { color: muted, fontSize: 10 },
      axisLine: { lineStyle: { color: rule } }
    },
    yAxis: {
      type: 'value',
      name: '显存 (GB)',
      max: 32,
      nameTextStyle: { color: muted, fontSize: 11 },
      axisLabel: { color: muted, fontSize: 11 },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } }
    },
    series: [
      {
        name: 'GPU 0 (Transformer)',
        type: 'line',
        data: [0.8, 0.8, 25.2, 25.2, 25.2, 25.2, 0.8],
        smooth: true,
        lineStyle: { color: accent, width: 2 },
        itemStyle: { color: accent },
        areaStyle: { color: accent + '22' }
      },
      {
        name: 'GPU 1 (LM)',
        type: 'line',
        data: [0, 30.9, 30.9, 30.9, 30.9, 30.9, 0.02],
        smooth: true,
        lineStyle: { color: accent2, width: 2 },
        itemStyle: { color: accent2 },
        areaStyle: { color: accent2 + '22' }
      },
      {
        name: 'GPU 2 (VAE)',
        type: 'line',
        data: [0, 7.5, 7.5, 7.5, 7.5, 7.5, 7.5],
        smooth: true,
        lineStyle: { color: muted, width: 2 },
        itemStyle: { color: muted },
        areaStyle: { color: muted + '22' }
      }
    ]
  });
  window.addEventListener('resize', function() { chart3.resize(); });

  // --- Chart 4: Quantization Size Comparison ---
  var chart4 = echarts.init(document.getElementById('chart-quant-size'), null, { renderer: 'svg' });
  chart4.setOption({
    animation: false,
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' }, appendToBody: true },
    legend: { top: 0, textStyle: { color: muted, fontSize: 12 } },
    grid: { left: '3%', right: '4%', bottom: '3%', top: 40, containLabel: true },
    xAxis: {
      type: 'category',
      data: ['Text Encoder\n(LM + Vision)', 'Transformer\n(Diffusion)', 'VAE\n(Video Decode)'],
      axisLabel: { color: muted, fontSize: 11 },
      axisLine: { lineStyle: { color: rule } }
    },
    yAxis: {
      type: 'value',
      name: '模型大小 (GB)',
      nameTextStyle: { color: muted, fontSize: 11 },
      axisLabel: { color: muted, fontSize: 11 },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } }
    },
    series: [
      {
        name: 'FP16 (原始)',
        type: 'bar',
        data: [52, 62, 19.6],
        itemStyle: { color: muted + '88' },
        barWidth: '30%',
        label: { show: true, position: 'top', color: muted, fontSize: 11 }
      },
      {
        name: 'INT8 (量化后)',
        type: 'bar',
        data: [26, 31, 9.8],
        itemStyle: { color: accent },
        barWidth: '30%',
        label: { show: true, position: 'top', color: accent, fontSize: 11, fontWeight: 'bold' }
      }
    ]
  });
  window.addEventListener('resize', function() { chart4.resize(); });
})();
