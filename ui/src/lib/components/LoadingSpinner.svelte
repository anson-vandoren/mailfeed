<script>
  export let size = 'md'; // 'sm' | 'md' | 'lg' | 'xl'
  export let variant = 'primary'; // 'primary' | 'secondary' | 'surface'
  export let text = '';

  const sizeClasses = {
    sm: 'w-4 h-4',
    md: 'w-6 h-6', 
    lg: 'w-8 h-8',
    xl: 'w-12 h-12'
  };

  const variantClasses = {
    primary: 'text-primary-500',
    secondary: 'text-secondary-500',
    surface: 'text-surface-500'
  };
</script>

<div class="flex items-center justify-center gap-2" role="status" aria-live="polite">
  <svg 
    class="{sizeClasses[size]} {variantClasses[variant]} animate-spin" 
    xmlns="http://www.w3.org/2000/svg" 
    fill="none" 
    viewBox="0 0 24 24"
    aria-hidden="true"
  >
    <circle 
      class="opacity-25" 
      cx="12" 
      cy="12" 
      r="10" 
      stroke="currentColor" 
      stroke-width="4"
    />
    <path 
      class="opacity-75" 
      fill="currentColor" 
      d="m4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
    />
  </svg>
  {#if text}
    <span class="text-{variant}-600 dark:text-{variant}-400">{text}</span>
  {/if}
  <span class="sr-only">Loading...</span>
</div>