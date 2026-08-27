import { defineStore } from 'pinia'

export const useUIStore = defineStore('ui', {
  state: () => ({
    tourCompleted: false,
    tourStep: 0
  }),
  actions: {
    completeTour() { this.tourCompleted = true },
    resetTour() { this.tourCompleted = false; this.tourStep = 0 }
  },
  persist: true
})