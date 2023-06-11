<script>
	import { user } from '../stores';
	import { login } from '../api';

	let email = '';
	let password = '';

	async function handleSubmit() {
		const res = await login(email, password);
		const { access_token, refresh_token } = await res.data;
		user.set({ email, token: access_token, refresh: refresh_token });
	}
</script>

<div class="grid h-screen place-items-center">
	<div class="card p-4">
		<form on:submit|preventDefault={handleSubmit}>
			<label for="email" class="label">Email</label>
			<input type="email" id="email" bind:value={email} class="input" />

			<label for="password" class="label">Password</label>
			<input type="password" id="password" bind:value={password} class="input" />

			<button type="submit" class="btn variant-filled-primary my-2">Login</button>
		</form>
	</div>
</div>
