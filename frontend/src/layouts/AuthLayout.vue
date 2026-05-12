<template>
  <div class="auth-layout">
    <!-- Ambient gradient glow -->
    <div class="bg-glow">
      <div class="glow-top" />
      <div class="glow-bottom" />
    </div>
    <div class="auth-container">
      <router-view />
    </div>
  </div>
</template>

<style scoped lang="scss">
.auth-layout {
  position: relative;
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #121212;
  padding: 24px;
  overflow: hidden;
}

/* Ambient glow */
.bg-glow {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.glow-top {
  position: absolute;
  width: 500px;
  height: 500px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(124, 58, 237, 0.08) 0%, transparent 70%);
  top: -200px;
  right: -120px;
  filter: blur(100px);
}

.glow-bottom {
  position: absolute;
  width: 400px;
  height: 400px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(16, 185, 129, 0.06) 0%, transparent 70%);
  bottom: -120px;
  left: -80px;
  filter: blur(100px);
}

.auth-container {
  position: relative;
  width: 100%;
  max-width: 420px;
}
</style>
