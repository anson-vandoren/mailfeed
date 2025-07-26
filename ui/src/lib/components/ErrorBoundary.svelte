<script>
  import { createEventDispatcher } from 'svelte';
  
  export let error = null;
  export let title = 'Something went wrong';
  export let showDetails = false;
  export let canRetry = true;
  export let retryText = 'Try Again';
  
  const dispatch = createEventDispatcher();
  
  function handleRetry() {
    error = null;
    dispatch('retry');
  }
  
  function toggleDetails() {
    showDetails = !showDetails;
  }
</script>

{#if error}
  <div class="card variant-filled-error p-6" role="alert">
    <div class="flex items-start gap-4">
      <!-- Error Icon -->
      <div class="text-2xl" aria-hidden="true">⚠️</div>
      
      <div class="flex-1">
        <!-- Error Title -->
        <h3 class="h4 text-error-50 mb-2">{title}</h3>
        
        <!-- User-friendly error message -->
        <div class="text-error-100 mb-4">
          {#if error.message}
            <p>{error.message}</p>
          {:else if error.response?.data?.error}
            <p>{error.response.data.error}</p>
          {:else if error.response?.status === 401}
            <p>Your session has expired. Please log in again.</p>
          {:else if error.response?.status === 403}
            <p>You don't have permission to perform this action.</p>
          {:else if error.response?.status === 404}
            <p>The requested resource was not found.</p>
          {:else if error.response?.status >= 500}
            <p>A server error occurred. Please try again later.</p>
          {:else if !navigator.onLine}
            <p>You appear to be offline. Please check your internet connection.</p>
          {:else}
            <p>An unexpected error occurred. Please try again.</p>
          {/if}
        </div>
        
        <!-- Action buttons -->
        <div class="flex gap-2 mb-4">
          {#if canRetry}
            <button 
              class="btn variant-filled-surface"
              on:click={handleRetry}
            >
              {retryText}
            </button>
          {/if}
          
          <button 
            class="btn variant-ghost-surface"
            on:click={toggleDetails}
          >
            {showDetails ? 'Hide Details' : 'Show Details'}
          </button>
        </div>
        
        <!-- Technical details (collapsible) -->
        {#if showDetails}
          <details class="mt-4">
            <summary class="cursor-pointer text-error-200 hover:text-error-100 mb-2">
              Technical Details
            </summary>
            <div class="bg-error-900/50 p-3 rounded-lg text-sm text-error-100 font-mono overflow-auto">
              <div><strong>Error:</strong> {error.name || 'Unknown'}</div>
              <div><strong>Message:</strong> {error.message || 'No message'}</div>
              {#if error.response}
                <div><strong>Status:</strong> {error.response.status} {error.response.statusText}</div>
                {#if error.response.data}
                  <div><strong>Response:</strong> {JSON.stringify(error.response.data, null, 2)}</div>
                {/if}
              {/if}
              {#if error.stack}
                <div><strong>Stack:</strong> <pre class="whitespace-pre-wrap">{error.stack}</pre></div>
              {/if}
            </div>
          </details>
        {/if}
      </div>
    </div>
  </div>
{:else}
  <slot />
{/if}