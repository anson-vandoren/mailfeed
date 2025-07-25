<script>
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { user } from '../stores';
	import { getSubscriptions, createSubscription, updateSubscription, deleteSubscription, validateFeed } from '../api';
	import Login from './login.svelte';

	let subscriptions = [];
	let loading = true;
	let subscriptionsLoading = false;
	let subscriptionsError = null;
	
	// Add subscription form
	let showAddForm = false;
	let addFormData = {
		url: '',
		frequency: 'daily',
		friendly_name: '',
		max_items: 10
	};
	let addFormLoading = false;
	let addFormError = null;
	let feedValidation = null;
	let isValidating = false;

	// Edit subscription state
	let editingSubscription = null;
	let editFormData = {};
	let editFormLoading = false;
	let editFormError = null;

	async function loadSubscriptions() {
		subscriptionsLoading = true;
		subscriptionsError = null;
		try {
			const localUser = get(user);
			if (localUser.token && localUser.userId) {
				subscriptions = await getSubscriptions(localUser);
			}
		} catch (e) {
			console.error('Failed to load subscriptions:', e);
			// Don't set error for auth failures since interceptor handles it
			if (e.response?.status !== 401 && e.response?.status !== 403) {
				subscriptionsError = e.message;
			}
		} finally {
			subscriptionsLoading = false;
		}
	}

	async function handleValidateFeed() {
		if (!addFormData.url.trim()) {
			feedValidation = null;
			return;
		}

		isValidating = true;
		try {
			const localUser = get(user);
			feedValidation = await validateFeed(localUser, addFormData.url.trim());
			
			// Auto-populate friendly name if not set and feed has a title
			if (!addFormData.friendly_name && feedValidation.valid && feedValidation.title) {
				addFormData.friendly_name = feedValidation.title;
			}
		} catch (e) {
			console.error('Failed to validate feed:', e);
			feedValidation = {
				valid: false,
				error: e.message
			};
		} finally {
			isValidating = false;
		}
	}

	async function handleAddSubscription() {
		if (!addFormData.url.trim()) {
			addFormError = 'Feed URL is required';
			return;
		}

		addFormLoading = true;
		addFormError = null;
		
		try {
			const localUser = get(user);
			const requestData = {
				url: addFormData.url.trim(),
				frequency: addFormData.frequency,
				friendly_name: addFormData.friendly_name.trim() || undefined,
				max_items: addFormData.max_items > 0 ? addFormData.max_items : undefined
			};
			
			await createSubscription(localUser, requestData);
			
			// Reset form and reload subscriptions
			addFormData = {
				url: '',
				frequency: 'daily',
				friendly_name: '',
				max_items: 10
			};
			feedValidation = null;
			showAddForm = false;
			await loadSubscriptions();
		} catch (e) {
			console.error('Failed to create subscription:', e);
			addFormError = e.message;
		} finally {
			addFormLoading = false;
		}
	}

	async function handleDeleteSubscription(subscription) {
		if (!confirm(`Are you sure you want to unsubscribe from "${subscription.subscription.friendly_name || subscription.feed.title}"?`)) {
			return;
		}

		try {
			const localUser = get(user);
			await deleteSubscription(localUser, subscription.subscription.id);
			await loadSubscriptions();
		} catch (e) {
			console.error('Failed to delete subscription:', e);
			alert('Failed to delete subscription: ' + e.message);
		}
	}

	function formatFrequency(frequency) {
		switch (frequency) {
			case 'realtime': return 'Real-time';
			case 'hourly': return 'Hourly';
			case 'daily': return 'Daily';
			default: return frequency;
		}
	}

	function formatLastSent(timestamp) {
		if (timestamp === 0) return 'Never';
		return new Date(timestamp * 1000).toLocaleDateString();
	}

	function startEditing(subscription) {
		editingSubscription = subscription.subscription.id;
		editFormData = {
			frequency: subscription.subscription.frequency,
			friendly_name: subscription.subscription.friendly_name || '',
			max_items: subscription.subscription.max_items || 10,
			is_active: subscription.subscription.is_active
		};
		editFormError = null;
	}

	function cancelEditing() {
		editingSubscription = null;
		editFormData = {};
		editFormError = null;
	}

	async function handleUpdateSubscription() {
		editFormLoading = true;
		editFormError = null;

		try {
			const localUser = get(user);
			await updateSubscription(localUser, editingSubscription, {
				frequency: editFormData.frequency,
				friendly_name: editFormData.friendly_name.trim() || undefined,
				max_items: editFormData.max_items > 0 ? editFormData.max_items : undefined,
				is_active: editFormData.is_active
			});

			editingSubscription = null;
			editFormData = {};
			await loadSubscriptions();
		} catch (e) {
			console.error('Failed to update subscription:', e);
			editFormError = e.message;
		} finally {
			editFormLoading = false;
		}
	}

	function getFeedStatus(feed) {
		// If there's a recent error (within last 24 hours)
		if (feed.error_time > 0 && (Date.now() / 1000 - feed.error_time) < 86400) {
			return { status: 'error', message: feed.error_message || 'Unknown error' };
		}
		
		// If never checked
		if (feed.last_checked === 0) {
			return { status: 'pending', message: 'Not yet checked' };
		}
		
		// If last check was more than 2 hours ago, consider it stale
		const hoursSinceCheck = (Date.now() / 1000 - feed.last_checked) / 3600;
		if (hoursSinceCheck > 2) {
			return { status: 'stale', message: `Last checked ${Math.floor(hoursSinceCheck)} hours ago` };
		}
		
		return { status: 'healthy', message: 'Working normally' };
	}

	function formatTimestamp(timestamp) {
		if (timestamp === 0) return 'Never';
		const date = new Date(timestamp * 1000);
		return date.toLocaleString();
	}

	onMount(async () => {
		if (get(user).token) {
			loading = true;
			await loadSubscriptions();
			loading = false;
		}
	});
</script>

{#if $user.token}
	<div class="container mx-auto p-4 max-w-4xl">
		<div class="mb-6">
			<h1 class="h2 mb-2">📰 Your RSS Feeds</h1>
			<p class="text-surface-600">Manage your RSS subscriptions and receive updates via Telegram.</p>
		</div>

		<!-- Add Subscription Section -->
		<div class="card p-6 mb-6">
			<div class="flex justify-between items-center mb-4">
				<h2 class="h3">Add New Feed</h2>
				<button 
					class="btn variant-filled-primary"
					on:click={() => {
						if (showAddForm) {
							// Reset form when canceling
							feedValidation = null;
							addFormData = {
								url: '',
								frequency: 'daily',
								friendly_name: '',
								max_items: 10
							};
						}
						showAddForm = !showAddForm;
					}}
				>
					{showAddForm ? 'Cancel' : '+ Add RSS Feed'}
				</button>
			</div>

			{#if showAddForm}
				<div class="card variant-ghost-surface p-4">
					{#if addFormError}
						<div class="alert variant-filled-error mb-4">
							<div>{addFormError}</div>
						</div>
					{/if}

					<form on:submit|preventDefault={handleAddSubscription} class="space-y-4">
						<div>
							<label class="label" for="url">
								<span>Feed URL <span class="text-error-500">*</span></span>
								<div class="input-group input-group-divider grid-cols-[1fr_auto]">
									<input 
										class="input" 
										type="url" 
										id="url"
										bind:value={addFormData.url}
										on:blur={handleValidateFeed}
										placeholder="https://example.com/feed.xml"
										required
										disabled={addFormLoading || isValidating}
									/>
									<button 
										type="button"
										class="btn variant-filled-secondary"
										on:click={handleValidateFeed}
										disabled={addFormLoading || isValidating || !addFormData.url.trim()}
									>
										{isValidating ? '⏳' : '🔍'}
									</button>
								</div>
							</label>
							
							<!-- Feed validation results -->
							{#if feedValidation}
								{#if feedValidation.valid}
									<div class="alert variant-filled-success mt-2">
										<div>
											<h4 class="h4">✅ Valid RSS Feed</h4>
											{#if feedValidation.title}
												<p><strong>Title:</strong> {feedValidation.title}</p>
											{/if}
											{#if feedValidation.description}
												<p><strong>Description:</strong> {feedValidation.description}</p>
											{/if}
										</div>
									</div>
								{:else}
									<div class="alert variant-filled-error mt-2">
										<div>
											<h4 class="h4">❌ Invalid Feed</h4>
											<p>{feedValidation.error || 'Unknown error occurred'}</p>
										</div>
									</div>
								{/if}
							{/if}
						</div>

						<div>
							<label class="label" for="friendly_name">
								<span>Display Name (optional)</span>
								<input 
									class="input" 
									type="text" 
									id="friendly_name"
									bind:value={addFormData.friendly_name}
									placeholder="My Favorite Blog"
									disabled={addFormLoading}
								/>
							</label>
						</div>

						<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
							<div>
								<label class="label" for="frequency">
									<span>Delivery Frequency</span>
									<select class="select" id="frequency" bind:value={addFormData.frequency} disabled={addFormLoading}>
										<option value="realtime">Real-time</option>
										<option value="hourly">Hourly</option>
										<option value="daily">Daily</option>
									</select>
								</label>
							</div>

							<div>
								<label class="label" for="max_items">
									<span>Max Items per Delivery</span>
									<input 
										class="input" 
										type="number" 
										id="max_items"
										bind:value={addFormData.max_items}
										min="1"
										max="50"
										disabled={addFormLoading}
									/>
								</label>
							</div>
						</div>

						<div class="flex gap-2">
							<button 
								type="submit" 
								class="btn variant-filled-primary"
								disabled={addFormLoading}
							>
								{addFormLoading ? 'Adding...' : 'Add Subscription'}
							</button>
							<button 
								type="button" 
								class="btn variant-ghost-surface"
								on:click={() => {
									showAddForm = false;
									feedValidation = null;
									addFormData = {
										url: '',
										frequency: 'daily',
										friendly_name: '',
										max_items: 10
									};
								}}
								disabled={addFormLoading}
							>
								Cancel
							</button>
						</div>
					</form>
				</div>
			{/if}
		</div>

		<!-- Subscriptions List Section -->
		<div class="card p-6">
			<h2 class="h3 mb-4">Your Subscriptions</h2>
			
			{#if loading}
				<p>Loading subscriptions...</p>
			{:else if subscriptionsError}
				<div class="alert variant-filled-error">
					<div>Error loading subscriptions: {subscriptionsError}</div>
				</div>
			{:else if subscriptions.length === 0}
				<div class="alert variant-ghost-surface">
					<div>
						<h4 class="h4">No subscriptions yet</h4>
						<p>Click "Add RSS Feed" above to subscribe to your first feed.</p>
					</div>
				</div>
			{:else}
				<div class="space-y-4">
					{#each subscriptions as subscription (subscription.subscription.id)}
						{@const status = getFeedStatus(subscription.feed)}
						<div class="card variant-ghost-surface p-4">
							{#if editingSubscription === subscription.subscription.id}
								<!-- Edit Mode -->
								<div class="space-y-4">
									{#if editFormError}
										<div class="alert variant-filled-error">
											<div>{editFormError}</div>
										</div>
									{/if}
									
									<div>
										<h4 class="h4 mb-4">Edit Subscription</h4>
										<div class="space-y-3">
											<div>
												<label class="label">
													<span>Display Name</span>
													<input 
														class="input" 
														type="text" 
														bind:value={editFormData.friendly_name}
														placeholder="My Favorite Blog"
														disabled={editFormLoading}
													/>
												</label>
											</div>
											<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
												<div>
													<label class="label">
														<span>Frequency</span>
														<select class="select" bind:value={editFormData.frequency} disabled={editFormLoading}>
															<option value="realtime">Real-time</option>
															<option value="hourly">Hourly</option>
															<option value="daily">Daily</option>
														</select>
													</label>
												</div>
												<div>
													<label class="label">
														<span>Max Items</span>
														<input 
															class="input" 
															type="number" 
															bind:value={editFormData.max_items}
															min="1"
															max="50"
															disabled={editFormLoading}
														/>
													</label>
												</div>
											</div>
											<div>
												<label class="flex items-center space-x-2">
													<input 
														class="checkbox" 
														type="checkbox" 
														bind:checked={editFormData.is_active}
														disabled={editFormLoading}
													/>
													<span>Active</span>
												</label>
											</div>
										</div>
									</div>
									
									<div class="flex gap-2">
										<button 
											class="btn variant-filled-primary"
											on:click={handleUpdateSubscription}
											disabled={editFormLoading}
										>
											{editFormLoading ? 'Saving...' : 'Save Changes'}
										</button>
										<button 
											class="btn variant-ghost-surface"
											on:click={cancelEditing}
											disabled={editFormLoading}
										>
											Cancel
										</button>
									</div>
								</div>
							{:else}
								<!-- View Mode -->
								<div class="flex justify-between items-start">
									<div class="flex-1">
										<div class="flex items-center gap-3 mb-2">
											<h4 class="h4">
												{subscription.subscription.friendly_name || subscription.feed.title || `Feed ${subscription.subscription.feed_id}`}
											</h4>
											<!-- Feed Health Status -->
											<span class="badge variant-soft-{status.status === 'healthy' ? 'success' : status.status === 'error' ? 'error' : status.status === 'stale' ? 'warning' : 'surface'}" title={status.message}>
												{#if status.status === 'healthy'}
													✅ Healthy
												{:else if status.status === 'error'}
													❌ Error
												{:else if status.status === 'stale'}
													⚠️ Stale
												{:else}
													⏳ Pending
												{/if}
											</span>
										</div>
										
										<!-- Feed URL -->
										<div class="text-sm text-surface-500 mb-2">
											<a href={subscription.feed.url} target="_blank" rel="noopener noreferrer" class="hover:underline">
												{subscription.feed.url}
											</a>
										</div>
										
										<!-- Subscription Details -->
										<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-2 text-sm text-surface-600 mb-2">
											<div>
												<span class="font-semibold">Frequency:</span> 
												{formatFrequency(subscription.subscription.frequency)}
											</div>
											<div>
												<span class="font-semibold">Max Items:</span> 
												{subscription.subscription.max_items || 'No limit'}
											</div>
											<div>
												<span class="font-semibold">Last Sent:</span> 
												{formatLastSent(subscription.subscription.last_sent_time)}
											</div>
											<div>
												<span class="font-semibold">Last Checked:</span> 
												{formatTimestamp(subscription.feed.last_checked)}
											</div>
										</div>
										
										<!-- Status and Error Info -->
										<div class="flex items-center gap-4">
											<span class="badge variant-soft-{subscription.subscription.is_active ? 'success' : 'warning'}">
												{subscription.subscription.is_active ? 'Active' : 'Inactive'}
											</span>
											{#if subscription.feed.error_time > 0 && subscription.feed.error_message}
												<div class="text-sm text-error-500">
													Last error: {subscription.feed.error_message}
												</div>
											{/if}
										</div>
									</div>
									<div class="flex gap-2 ml-4">
										<button 
											class="btn-icon variant-filled-secondary"
											on:click={() => startEditing(subscription)}
											title="Edit subscription"
										>
											✏️
										</button>
										<button 
											class="btn-icon variant-filled-error"
											on:click={() => handleDeleteSubscription(subscription)}
											title="Delete subscription"
										>
											🗑️
										</button>
									</div>
								</div>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</div>
{:else}
	<Login />
{/if}
