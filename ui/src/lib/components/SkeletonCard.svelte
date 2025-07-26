<script>
  export let rows = 3;
  export let showTitle = true;
  export let showActions = true;
</script>

<div class="card variant-ghost-surface p-4 animate-pulse" aria-hidden="true">
  <div class="flex justify-between items-start">
    <!-- Main Content Area -->
    <div class="flex-1 mr-4">
      {#if showTitle}
        <!-- Title Skeleton -->
        <div class="h-6 bg-surface-300 dark:bg-surface-600 rounded w-3/4 mb-2"></div>
      {/if}
      
      <!-- URL Skeleton -->
      <div class="h-4 bg-surface-200 dark:bg-surface-700 rounded w-1/2 mb-3"></div>
      
      <!-- Details Grid Skeleton -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-3">
        <div class="space-y-2">
          {#each Array(rows) as _}
            <div class="h-4 bg-surface-200 dark:bg-surface-700 rounded w-full"></div>
          {/each}
        </div>
        <div class="space-y-2">
          {#each Array(rows) as _}
            <div class="h-4 bg-surface-200 dark:bg-surface-700 rounded w-5/6"></div>
          {/each}
        </div>
      </div>
    </div>

    {#if showActions}
      <!-- Status & Actions Area -->
      <div class="flex flex-col items-end gap-3">
        <!-- Status Badges -->
        <div class="flex flex-col gap-2 items-end">
          <div class="h-6 bg-surface-300 dark:bg-surface-600 rounded-full w-16"></div>
          <div class="h-6 bg-surface-300 dark:bg-surface-600 rounded-full w-20"></div>
        </div>

        <!-- Action Buttons -->
        <div class="flex gap-2">
          <div class="w-8 h-8 bg-surface-300 dark:bg-surface-600 rounded"></div>
          <div class="w-8 h-8 bg-surface-300 dark:bg-surface-600 rounded"></div>
        </div>
      </div>
    {/if}
  </div>
</div>