<script setup lang="ts">
import type { StageRun } from "../types";

defineProps<{ stages: StageRun[]; label?: string }>();
</script>

<template>
  <div class="ledger">
    <i
      v-for="(s, i) in stages"
      :key="i"
      class="step"
      :class="s.state"
      :title="s.name + (s.ms ? ' · ' + s.ms + ' ms' : '')"
    ></i>
    <span v-if="label" class="label">{{ label }}</span>
  </div>
</template>
