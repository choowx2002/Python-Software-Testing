import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

interface TourStep {
  target: string
  title: string
  content: string
  position?: 'top' | 'bottom' | 'left' | 'right'
}

export function useDashboardTour() {
  const { t } = useI18n()

  return computed<TourStep[]>(() => [
    {
      target: '[data-tour="import-project"]',
      title: t('tour.dashboard.step1Title'),
      content: t('tour.dashboard.step1Desc'),
      position: 'right'
    },
    {
      target: '[data-tour="clone-repo"]',
      title: t('tour.dashboard.step2Title'),
      content: t('tour.dashboard.step2Desc'),
      position: 'right'
    },
    {
      target: '[data-tour="project-list"]',
      title: t('tour.dashboard.step3Title'),
      content: t('tour.dashboard.step3Desc'),
      position: 'left'
    }
  ])
}

export function useProjectTour() {
  const { t } = useI18n()

  return computed<TourStep[]>(() => [
    {
      target: '[data-tour="overview-tab"]',
      title: t('tour.project.step1Title'),
      content: t('tour.project.step1Desc'),
      position: 'bottom'
    },
    {
      target: '[data-tour="generate-tab"]',
      title: t('tour.project.step2Title'),
      content: t('tour.project.step2Desc'),
      position: 'left'
    },
    {
      target: '[data-tour="execute-tab"]',
      title: t('tour.project.step3Title'),
      content: t('tour.project.step3Desc'),
      position: 'left'
    },
    {
      target: '[data-tour="coverage-tab"]',
      title: t('tour.project.step4Title'),
      content: t('tour.project.step4Desc'),
      position: 'left'
    },
    {
      target: '[data-tour="history-tab"]',
      title: t('tour.project.step5Title'),
      content: t('tour.project.step5Desc'),
      position: 'left'
    }
  ])
}