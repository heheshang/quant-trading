<template>
  <div class="auth-layout">
    <div class="auth-container">
      <div class="auth-header">
        <div class="auth-logo">
          <svg width="32" height="32" viewBox="0 0 32 32" fill="none">
            <rect width="32" height="32" rx="8" fill="var(--color-accent)" />
            <path d="M8 22V10L16 6L24 10V22L16 26L8 22Z" stroke="white" stroke-width="1.5" fill="none" />
            <path d="M12 18L14 20L20 14" stroke="white" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
          <span class="auth-title">Quant Trading</span>
        </div>
        <p class="auth-subtitle">Quantitative Trading System</p>
      </div>
      <div class="auth-card">
        <slot />
      </div>
      <div class="auth-footer">
        <p>&copy; 2026 Quant Trading System. All rights reserved.</p>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.auth-layout {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-bg);
  padding: 24px;
}

.auth-container {
  width: 100%;
  max-width: 400px;
}

.auth-header {
  text-align: center;
  margin-bottom: 32px;
}

.auth-logo {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-bottom: 8px;
}

.auth-title {
  font-size: 22px;
  font-weight: 600;
  color: var(--color-text-primary);
  letter-spacing: -0.3px;
}

.auth-subtitle {
  font-size: 13px;
  color: var(--color-text-tertiary);
  margin: 0;
}

.auth-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 12px;
  padding: 32px;
}

.auth-footer {
  text-align: center;
  margin-top: 24px;

  p {
    font-size: 12px;
    color: var(--color-text-tertiary);
    margin: 0;
  }
}
</style>
