<script setup lang="ts">
import NcmSettings from '../NcmSettings.vue'
import { store } from '../../store'

const adminNcmState = {
  get badge() { return store.ncmBadge },
  get badgeClass() { return store.ncmBadgeClass },
  get activeTab() { return store.ncmActiveTab },
  get cookie() { return store.ncmCookie },
  get phone() { return store.ncmPhone },
  get password() { return store.ncmPassword },
  get result() { return store.ncmResult },
  get resultType() { return store.ncmResultType },
}
</script>

<template>
  <NcmSettings
    api-prefix="/api/admin/ncm"
    :state-getter="() => adminNcmState"
    :state-setter="(p: Record<string, any>) => Object.assign(store, p)"
  />
</template>
