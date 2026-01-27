import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'home',
      component: () => import('../views/Home.vue')
    },
    {
      path: '/register',
      name: 'register',
      component: () => import('../views/Register.vue'),
      meta: { requiresGuest: true }
    },
    {
      path: '/login',
      name: 'login',
      component: () => import('../views/Login.vue'),
      meta: { requiresGuest: true }
    },
    {
      path: '/dashboard',
      name: 'dashboard',
      component: () => import('../views/Dashboard.vue'),
      meta: { requiresAuth: true },
      children: [
        {
          path: '',
          name: 'dashboard-home',
          component: () => import('../views/DashboardHome.vue')
        },
        {
          path: 'profile',
          name: 'profile',
          component: () => import('../views/Profile.vue')
        },
        {
          path: 'packages',
          name: 'packages',
          component: () => import('../views/Packages.vue')
        },
        {
          path: 'orders',
          name: 'orders',
          component: () => import('../views/Orders.vue')
        },
        {
          path: 'subscription',
          name: 'subscription',
          component: () => import('../views/Subscription.vue')
        },
        {
          path: 'referral',
          name: 'referral',
          component: () => import('../views/Referral.vue')
        }
      ]
    }
  ]
})

// Navigation guards
router.beforeEach((to, from, next) => {
  const authStore = useAuthStore()
  
  if (to.meta.requiresAuth && !authStore.isAuthenticated) {
    next({ name: 'login', query: { redirect: to.fullPath } })
  } else if (to.meta.requiresGuest && authStore.isAuthenticated) {
    next({ name: 'dashboard' })
  } else {
    next()
  }
})

export default router
