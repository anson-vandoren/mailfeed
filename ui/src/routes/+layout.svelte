<script>
	import '@skeletonlabs/skeleton/themes/theme-crimson.css';
	import '@skeletonlabs/skeleton/styles/skeleton.css';
	import { LightSwitch } from '@skeletonlabs/skeleton';
	import '../app.postcss';
	import { AppBar, AppShell } from '@skeletonlabs/skeleton';
	import { user, toasts, appState, addToast } from '../stores';
	import { logout } from '../api';
	import Toast from '../lib/components/Toast.svelte';
	import { onMount } from 'svelte';

	let isLoggingOut = false;

	async function doLogout() {
		if (isLoggingOut) return;
		
		isLoggingOut = true;
		try {
			await logout();
			addToast({
				message: 'Successfully logged out',
				type: 'success'
			});
		} catch (error) {
			addToast({
				message: 'Logout failed, but session was cleared',
				type: 'warning'
			});
		} finally {
			isLoggingOut = false;
		}
	}

	// Handle visibility change to detect when user returns
	onMount(() => {
		function handleVisibilityChange() {
			if (!document.hidden && $user.userId) {
				// User returned to the app - could refresh data here
				console.log('User returned to app');
			}
		}

		document.addEventListener('visibilitychange', handleVisibilityChange);

		return () => {
			document.removeEventListener('visibilitychange', handleVisibilityChange);
		};
	});
</script>

<AppShell>
	<svelte:fragment slot="header">
		<AppBar>
			<svelte:fragment slot="lead">
				<a href="/" class="h2 flex items-center gap-2">
					📰 <span class="hidden sm:inline">MailFeed</span>
				</a>
			</svelte:fragment>
			<svelte:fragment slot="trail">
				<!-- Connection status indicator -->
				{#if !$appState.isOnline}
					<span class="badge variant-filled-warning text-xs">Offline</span>
				{/if}
				
				<!-- Loading indicator -->
				{#if $appState.isLoading}
					<div class="flex items-center gap-1">
						<div class="w-2 h-2 bg-primary-500 rounded-full animate-pulse"></div>
						<span class="text-xs hidden sm:inline">Loading...</span>
					</div>
				{/if}
				
				<LightSwitch />
				
				{#if $user.userId}
					<!-- Mobile menu button for small screens -->
					<div class="sm:hidden">
						<button class="btn-icon variant-outline-surface" title="Menu">
							☰
						</button>
					</div>
					
					<!-- Desktop navigation -->
					<div class="hidden sm:flex items-center gap-2">
						<a href="/config" class="btn-sm variant-outline-tertiary">Configuration</a>
						<a href="/settings" class="btn-sm variant-outline-secondary">Settings</a>
						<button 
							on:click={doLogout} 
							class="btn-sm variant-ghost-primary"
							disabled={isLoggingOut}
						>
							{#if isLoggingOut}
								<svg class="w-3 h-3 animate-spin mr-1" fill="none" viewBox="0 0 24 24">
									<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/>
									<path class="opacity-75" fill="currentColor" d="m4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"/>
								</svg>
							{/if}
							Logout
						</button>
					</div>
				{/if}
			</svelte:fragment>
		</AppBar>
	</svelte:fragment>
	
	<!-- Main content with proper spacing and responsive design -->
	<div class="min-h-screen">
		<slot />
	</div>
</AppShell>

<!-- Toast notifications container -->
<div class="fixed top-4 right-4 z-50 flex flex-col gap-2">
	{#each $toasts as toast (toast.id)}
		<Toast
			message={toast.message}
			type={toast.type}
			duration={toast.duration}
			dismissible={toast.dismissible}
			on:dismiss={() => {
				toasts.update(t => t.filter(t => t.id !== toast.id));
			}}
		/>
	{/each}
</div>

<!-- Global styles for responsive improvements -->
<style>
	:global(body) {
		overflow-x: hidden;
	}
	
	:global(.container) {
		max-width: 100%;
		padding-left: 1rem;
		padding-right: 1rem;
	}
	
	@media (min-width: 640px) {
		:global(.container) {
			padding-left: 1.5rem;
			padding-right: 1.5rem;
		}
	}
	
	@media (min-width: 1024px) {
		:global(.container) {
			padding-left: 2rem;
			padding-right: 2rem;
		}
	}
</style>
