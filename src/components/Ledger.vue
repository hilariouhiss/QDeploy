<script setup lang="ts">
import { formatMs } from "../store";
import type { StageRun } from "../types";

defineProps<{ stages: StageRun[]; label?: string; negative?: boolean }>();
</script>

<template>
  <div class="railwrap">
    <div class="rail" aria-hidden="true">
      <i
        v-for="(s, i) in stages"
        :key="i"
        :class="s.state"
        :title="s.name + (s.ms ? ' · ' + formatMs(s.ms) : '')"
      ></i>
    </div>
    <span v-if="label" class="rail-label" :class="{ neg: negative }">{{ label }}</span>
  </div>
</template>
