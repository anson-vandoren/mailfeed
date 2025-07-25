<script>
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { user } from '../../stores';
	import { getUserDetails, updateTelegramSettings, testTelegram } from '../../api';

	/** @type {import('./$types').PageData} */
	export let data = {};
	
	let userDetails = null;
	let loading = true;
	let error = null;
	
	// Telegram settings
	let telegramChatId = '';
	let telegramUsername = '';
	let telegramLoading = false;
	let telegramError = null;
	let telegramSuccess = false;

	async function loadUserDetails() {
		try {
			const localUser = get(user);
			if (localUser.token && localUser.userId) {
				userDetails = await getUserDetails(localUser);
				// Populate Telegram settings if they exist
				if (userDetails.telegram_chat_id) {
					telegramChatId = userDetails.telegram_chat_id;
				}
				if (userDetails.telegram_username) {
					telegramUsername = userDetails.telegram_username;
				}
			}
		} catch (e) {
			console.error('Failed to load user details:', e);
			// Don't set error for auth failures since interceptor handles it
			if (e.response?.status !== 401 && e.response?.status !== 403) {
				error = e.message;
			}
		}
	}


	async function handleUpdateTelegram() {
		if (!telegramChatId.trim()) {
			telegramError = 'Chat ID is required';
			return;
		}

		telegramLoading = true;
		telegramError = null;
		telegramSuccess = false;

		try {
			const localUser = get(user);
			await updateTelegramSettings(
				localUser, 
				telegramChatId.trim(), 
				telegramUsername.trim() || undefined
			);
			
			telegramSuccess = true;
			setTimeout(() => {
				telegramSuccess = false;
			}, 3000);
		} catch (e) {
			console.error('Failed to update Telegram settings:', e);
			telegramError = e.message;
		} finally {
			telegramLoading = false;
		}
	}

	async function handleTestTelegram() {
		telegramLoading = true;
		telegramError = null;
		telegramSuccess = false;

		try {
			const localUser = get(user);
			const result = await testTelegram(localUser);
			
			if (result.success) {
				telegramSuccess = true;
				setTimeout(() => {
					telegramSuccess = false;
				}, 5000);
			} else {
				telegramError = result.error || 'Test failed';
			}
		} catch (e) {
			console.error('Failed to test Telegram:', e);
			telegramError = e.message;
		} finally {
			telegramLoading = false;
		}
	}


	onMount(async () => {
		loading = true;
		await loadUserDetails();
		loading = false;
	});
</script>

<div class="container mx-auto p-4 max-w-4xl">
	<h1 class="h2 mb-6">Settings</h1>
	
	<!-- Account Details Section -->
	<div class="card p-6 mb-6">
		<h2 class="h3 mb-4">Account Details</h2>
		
		{#if loading}
			<p>Loading account details...</p>
		{:else if error}
			<p class="text-error-500">Error: {error}</p>
		{:else if userDetails}
			<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
				<div>
					<label class="label">Login Email</label>
					<p class="text-surface-700">{userDetails.loginEmail}</p>
				</div>
				<div>
					<label class="label">Send Email</label>
					<p class="text-surface-700">{userDetails.sendEmail}</p>
				</div>
				<div>
					<label class="label">Account Created</label>
					<p class="text-surface-700">{new Date(userDetails.createdAt * 1000).toLocaleDateString()}</p>
				</div>
				<div>
					<label class="label">Daily Send Time</label>
					<p class="text-surface-700">{userDetails.dailySendTime}</p>
				</div>
			</div>
		{/if}
	</div>

	<!-- Telegram Configuration Section -->
	<div class="card p-6 mb-6">
		<h2 class="h3 mb-4">Telegram Configuration</h2>
		<p class="text-sm text-surface-600 mb-4">
			Configure your Telegram chat ID to receive RSS feed updates via Telegram bot.
		</p>
		
		{#if telegramError}
			<div class="alert variant-filled-error mb-4">
				<div>{telegramError}</div>
			</div>
		{/if}

		{#if telegramSuccess}
			<div class="alert variant-filled-success mb-4">
				<div>✅ Telegram settings updated successfully!</div>
			</div>
		{/if}

		<form on:submit|preventDefault={handleUpdateTelegram} class="space-y-4">
			<div>
				<label class="label" for="telegram_chat_id">
					<span>Telegram Chat ID <span class="text-error-500">*</span></span>
					<input 
						class="input" 
						type="text" 
						id="telegram_chat_id"
						placeholder="123456789"
						bind:value={telegramChatId}
						disabled={telegramLoading}
						required
					/>
					<div class="text-xs text-surface-500 mt-1">
						Message your bot and check the bot logs, or use @userinfobot to get your chat ID
					</div>
				</label>
			</div>

			<div>
				<label class="label" for="telegram_username">
					<span>Telegram Username (optional)</span>
					<input 
						class="input" 
						type="text" 
						id="telegram_username"
						placeholder="@username"
						bind:value={telegramUsername}
						disabled={telegramLoading}
					/>
				</label>
			</div>

			<div class="flex gap-2">
				<button 
					type="submit" 
					class="btn variant-filled-primary"
					disabled={telegramLoading}
				>
					{telegramLoading ? 'Updating...' : 'Update Telegram Settings'}
				</button>
				
				{#if telegramChatId}
					<button 
						type="button"
						class="btn variant-filled-secondary"
						on:click={handleTestTelegram}
						disabled={telegramLoading}
					>
						{telegramLoading ? 'Testing...' : '🧪 Send Test Message'}
					</button>
				{/if}
			</div>
		</form>

		<!-- Chat ID Discovery Help -->
		<div class="card variant-ghost-surface p-4 mt-4">
			<h4 class="h4 mb-2">📱 How to find your Chat ID:</h4>
			<ol class="list-decimal list-inside space-y-1 text-sm">
				<li>Start a conversation with your bot</li>
				<li>Send any message to the bot</li>
				<li>Check the backend logs for your chat ID, or</li>
				<li>Use @userinfobot on Telegram - send it any message and it will reply with your user info</li>
				<li>Copy the chat ID and paste it above</li>
			</ol>
		</div>
	</div>

</div>
