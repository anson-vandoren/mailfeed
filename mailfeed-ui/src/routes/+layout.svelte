<script>
	import '@skeletonlabs/skeleton/themes/theme-skeleton.css';
	import '@skeletonlabs/skeleton/styles/skeleton.css';
	import { LightSwitch } from '@skeletonlabs/skeleton';
	import '../app.postcss';
	import { AppBar, AppShell } from '@skeletonlabs/skeleton';
	import { user } from '../stores';
	import { logout } from '../api';

	function doLogout() {
		logout();
		user.set({});
	}
</script>

<AppShell>
	<svelte:fragment slot="header">
		<AppBar>
			<svelte:fragment slot="lead">
				<a href="/" class="h2">MailFeed</a>
			</svelte:fragment>
			<svelte:fragment slot="trail">
				<LightSwitch />
				{#if $user.token}
					<button on:click={doLogout} class="btn-sm variant-ghost-primary">Logout</button>
				{/if}
			</svelte:fragment>
		</AppBar>
	</svelte:fragment>
	<slot />
</AppShell>
