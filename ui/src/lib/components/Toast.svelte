<script>
  import { createEventDispatcher } from 'svelte';
  import { slide } from 'svelte/transition';
  
  export let message = '';
  export let type = 'info'; // 'success' | 'error' | 'warning' | 'info'
  export let duration = 5000; // Auto-dismiss after 5 seconds
  export let dismissible = true;
  export let visible = true;
  
  const dispatch = createEventDispatcher();
  
  let timeoutId = null;
  
  $: if (visible && duration > 0) {
    if (timeoutId) clearTimeout(timeoutId);
    timeoutId = setTimeout(() => {
      handleDismiss();
    }, duration);
  }
  
  function handleDismiss() {
    if (timeoutId) clearTimeout(timeoutId);
    visible = false;
    dispatch('dismiss');
  }
  
  const typeConfig = {
    success: {
      icon: '✅',
      classes: 'variant-filled-success'
    },
    error: {
      icon: '❌', 
      classes: 'variant-filled-error'
    },
    warning: {
      icon: '⚠️',
      classes: 'variant-filled-warning'
    },
    info: {
      icon: 'ℹ️',
      classes: 'variant-filled-secondary'
    }
  };
</script>

{#if visible && message}
  <div 
    class="toast {typeConfig[type].classes} flex items-center gap-3 p-4 rounded-lg shadow-lg max-w-md"
    transition:slide={{ duration: 200 }}
    role="alert"
    aria-live="polite"
  >
    <div class="text-lg" aria-hidden="true">
      {typeConfig[type].icon}
    </div>
    
    <div class="flex-1">
      <p class="font-medium">{message}</p>
    </div>
    
    {#if dismissible}
      <button 
        class="btn-icon variant-ghost hover:variant-soft-surface"
        on:click={handleDismiss}
        aria-label="Dismiss notification"
      >
        ✕
      </button>
    {/if}
  </div>
{/if}

<style>
  .toast {
    position: fixed;
    top: 1rem;
    right: 1rem;
    z-index: 1000;
  }
  
  @media (max-width: 640px) {
    .toast {
      left: 1rem;
      right: 1rem;
      max-width: none;
    }
  }
</style>