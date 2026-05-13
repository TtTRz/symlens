<template>
  <div class="app">
    <h1>{{ title }}</h1>
    <ChildComponent :count="count" @increment="increment" />
  </div>
</template>

<script lang="ts">
import { ref, computed } from 'vue'
import ChildComponent from './ChildComponent.vue'

interface User {
  name: string
  age: number
}

type Status = 'active' | 'inactive'

export default {
  components: { ChildComponent },
  setup() {
    const count = ref(0)
    const title = ref('Hello Vue')

    function increment(): void {
      count.value++
    }

    const doubleCount = computed(() => count.value * 2)

    return { count, title, increment, doubleCount }
  }
}
</script>

<script setup lang="ts">
import { ref as setupRef } from 'vue'

const setupCount = setupRef(0)

function setupIncrement(): void {
  setupCount.value++
}

enum Color {
  Red,
  Green,
  Blue,
}
</script>

<style scoped>
.app {
  padding: 20px;
}
</style>
