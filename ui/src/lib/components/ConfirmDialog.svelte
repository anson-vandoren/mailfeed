<script>
  import { createEventDispatcher } from 'svelte';
  import { Modal } from '@skeletonlabs/skeleton';
  
  export let open = false;
  export let title = 'Confirm Action';
  export let message = 'Are you sure you want to proceed?';
  export let confirmText = 'Confirm';
  export let cancelText = 'Cancel';
  export let variant = 'warning'; // 'warning' | 'error' | 'primary'
  export let loading = false;
  
  const dispatch = createEventDispatcher();
  
  function handleConfirm() {
    dispatch('confirm');
  }
  
  function handleCancel() {
    if (!loading) {
      open = false;
      dispatch('cancel');
    }
  }
  
  const variantClasses = {
    warning: 'variant-filled-warning',
    error: 'variant-filled-error', 
    primary: 'variant-filled-primary'
  };
  
  const iconMap = {
    warning: '⚠️',
    error: '🗑️',
    primary: 'ℹ️'
  };
</script>

{#if open}
  <Modal bind:open regionBackdrop="backdrop-blur-sm">
    <div class="card p-6 max-w-md mx-auto">
      <div class="flex items-center gap-4 mb-4">
        <div class="text-2xl" aria-hidden="true">{iconMap[variant]}</div>
        <h3 class="h3">{title}</h3>
      </div>
      
      <p class="text-surface-600 dark:text-surface-400 mb-6">
        {message}
      </p>
      
      <div class="flex gap-2 justify-end">
        <button 
          class="btn variant-ghost-surface"
          on:click={handleCancel}
          disabled={loading}
        >
          {cancelText}
        </button>
        <button 
          class="btn {variantClasses[variant]}"
          on:click={handleConfirm}
          disabled={loading}
        >
          {#if loading}
            <svg class="w-4 h-4 animate-spin mr-2" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/>
              <path class="opacity-75" fill="currentColor" d="m4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"/>
            </svg>
          {/if}
          {confirmText}
        </button>
      </div>
    </div>
  </Modal>
{/if}