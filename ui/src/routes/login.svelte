<script>
	import { onMount } from 'svelte';
	import { user } from '../stores';
	import { login } from '../api';

	let email = '';
	let password = '';
	let emailInput;

	onMount(() => {
		emailInput?.focus();
	});

	async function handleSubmit() {
		const res = await login(email, password);
		const { user_id } = await res.data;
		user.set({ email, userId: user_id });
	}
</script>

<div class="grid h-screen place-items-center">
	<div class="card p-4">
		<form on:submit|preventDefault={handleSubmit}>
			<label for="email" class="label">Email</label>
			<input type="email" id="email" bind:value={email} bind:this={emailInput} class="input" />

			<label for="password" class="label">Password</label>
			<input type="password" id="password" bind:value={password} class="input" />

			<button type="submit" class="btn variant-filled-primary my-2">Login</button>
		</form>
	</div>
</div>
