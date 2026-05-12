<template>
  <div class="placeholder-view">
    <div class="page-header">
      <h1>Backtest</h1>
    </div>
    <el-card class="placeholder-card" shadow="never">
      <el-empty description="Backtest engine coming soon" :image-size="80">
        <template #image>
          <el-icon :size="48" color="var(--color-text-tertiary)"><Odometer /></el-icon>
        </template>
      </el-empty>
    </el-card>
  </div>
</template>

<style scoped lang="scss">
.placeholder-view { max-width: 1344px; }
.page-header { margin-bottom: 24px; }
.page-header h1 {
  font-size: 24px; font-weight: 600; color: var(--color-text-primary); margin: 0;
}
.placeholder-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 40px;
}
</style>
