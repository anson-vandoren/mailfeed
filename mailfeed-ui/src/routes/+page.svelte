<script>
	import { user } from '../stores';
	import Login from './login.svelte';
</script>

{#if $user.token}
	<p>Logged in as {$user.email}</p>
{:else}
	<Login />
{/if}
