<script>
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { user } from '../../stores';
	import { getConfiguration, updateConfiguration, bulkUpdateConfiguration } from '../../api';
	import Login from '../login.svelte';

	let config = {};
	let schema = [];
	let loading = true;
	let saving = false;
	let error = null;
	let successMessage = null;
	let pendingChanges = {};

	// Group configurations by category
	let configByCategory = {};

	async function loadConfiguration() {
		loading = true;
		error = null;
		
		try {
			const localUser = get(user);
			if (localUser.userId) {
				const response = await getConfiguration(localUser);
				config = response.config;
				schema = response.schema;
				
				// Group by category
				configByCategory = {};
				schema.forEach(item => {
					if (!configByCategory[item.category]) {
						configByCategory[item.category] = [];
					}
					configByCategory[item.category].push(item);
				});
			}
		} catch (e) {
			console.error('Failed to load configuration:', e);
			if (e.response?.status !== 401 && e.response?.status !== 403) {
				error = e.message;
			}
		} finally {
			loading = false;
		}
	}

	async function handleSaveAll() {
		if (Object.keys(pendingChanges).length === 0) {
			return;
		}

		saving = true;
		error = null;
		successMessage = null;

		try {
			const localUser = get(user);
			const response = await bulkUpdateConfiguration(localUser, { updates: pendingChanges });
			
			// Update config with new values
			Object.keys(response).forEach(key => {
				config[key] = response[key];
			});
			
			pendingChanges = {};
			successMessage = 'Configuration saved successfully!';
			
			// Clear success message after 3 seconds
			setTimeout(() => {
				successMessage = null;
			}, 3000);
		} catch (e) {
			console.error('Failed to save configuration:', e);
			error = e.message;
		} finally {
			saving = false;
		}
	}

	function handleConfigChange(key, value) {
		pendingChanges[key] = value;
		pendingChanges = { ...pendingChanges }; // Trigger reactivity
	}

	function resetChanges() {
		pendingChanges = {};
	}

	function formatCategoryName(category) {
		return category.split('_').map(word => 
			word.charAt(0).toUpperCase() + word.slice(1)
		).join(' ');
	}

	function getCurrentValue(key) {
		return pendingChanges[key] !== undefined ? pendingChanges[key] : config[key]?.value || '';
	}

	function hasChanges() {
		return Object.keys(pendingChanges).length > 0;
	}

	onMount(async () => {
		if (get(user).userId) {
			await loadConfiguration();
		}
	});
</script>

{#if $user.userId}
	<div class="container mx-auto p-4 max-w-4xl">
		<div class="mb-6">
			<h1 class="h2 mb-2">⚙️ Configuration</h1>
			<p class="text-surface-600">Customize your mailfeed experience with these settings.</p>
		</div>

		{#if loading}
			<div class="card p-6">
				<p>Loading configuration...</p>
			</div>
		{:else if error}
			<div class="alert variant-filled-error mb-4">
				<div>Error loading configuration: {error}</div>
			</div>
		{:else}
			<!-- Action Bar -->
			{#if hasChanges()}
				<div class="card variant-ghost-warning p-4 mb-6">
					<div class="flex justify-between items-center">
						<div>
							<h4 class="h4">Unsaved Changes</h4>
							<p class="text-sm">You have {Object.keys(pendingChanges).length} unsaved changes.</p>
						</div>
						<div class="flex gap-2">
							<button 
								class="btn variant-ghost-surface"
								on:click={resetChanges}
								disabled={saving}
							>
								Reset
							</button>
							<button 
								class="btn variant-filled-primary"
								on:click={handleSaveAll}
								disabled={saving}
							>
								{saving ? 'Saving...' : 'Save All Changes'}
							</button>
						</div>
					</div>
				</div>
			{/if}

			{#if successMessage}
				<div class="alert variant-filled-success mb-4">
					<div>{successMessage}</div>
				</div>
			{/if}

			{#if error}
				<div class="alert variant-filled-error mb-4">
					<div>Error: {error}</div>
				</div>
			{/if}

			<!-- Configuration Sections -->
			{#each Object.entries(configByCategory) as [category, items]}
				<div class="card p-6 mb-6">
					<h2 class="h3 mb-4">{formatCategoryName(category)}</h2>
					
					<div class="space-y-4">
						{#each items as schemaItem}
							{@const hasChanged = pendingChanges[schemaItem.key] !== undefined}
							
							<div class="border-l-4 border-l-{hasChanged ? 'warning-500' : 'transparent'} pl-4">
								<div class="mb-2">
									<label class="label flex items-center gap-2">
										<span class="font-semibold">{schemaItem.display_name}</span>
										{#if hasChanged}
											<span class="badge variant-soft-warning text-xs">Modified</span>
										{/if}
									</label>
									<p class="text-sm text-surface-600">{schemaItem.description}</p>
								</div>
								
								{#if schemaItem.config_type === 'string'}
									<input 
										class="input"
										type="text"
										value={getCurrentValue(schemaItem.key)}
										on:input={(e) => handleConfigChange(schemaItem.key, e.target.value)}
										disabled={saving}
									/>
								{:else if schemaItem.config_type === 'number'}
									<input 
										class="input"
										type="number"
										value={getCurrentValue(schemaItem.key)}
										on:input={(e) => handleConfigChange(schemaItem.key, e.target.value)}
										min={schemaItem.validation?.min}
										max={schemaItem.validation?.max}
										disabled={saving}
									/>
								{:else if schemaItem.config_type === 'boolean'}
									<label class="flex items-center space-x-2">
										<input 
											class="checkbox"
											type="checkbox"
											checked={getCurrentValue(schemaItem.key) === 'true'}
											on:change={(e) => handleConfigChange(schemaItem.key, e.target.checked ? 'true' : 'false')}
											disabled={saving}
										/>
										<span>Enable</span>
									</label>
								{:else if schemaItem.config_type === 'select'}
									<select 
										class="select"
										value={getCurrentValue(schemaItem.key)}
										on:change={(e) => handleConfigChange(schemaItem.key, e.target.value)}
										disabled={saving}
									>
										{#each schemaItem.options as option}
											<option value={option.value}>{option.label}</option>
										{/each}
									</select>
								{/if}
								
								{#if schemaItem.validation}
									<div class="text-xs text-surface-500 mt-1">
										{#if schemaItem.validation.min !== null && schemaItem.validation.max !== null}
											Range: {schemaItem.validation.min} - {schemaItem.validation.max}
										{:else if schemaItem.validation.min !== null}
											Minimum: {schemaItem.validation.min}
										{:else if schemaItem.validation.max !== null}
											Maximum: {schemaItem.validation.max}
										{/if}
										{#if schemaItem.validation.required}
											• Required
										{/if}
									</div>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{/each}

			<!-- Save Button at Bottom -->
			{#if hasChanges()}
				<div class="card p-4">
					<div class="flex justify-end gap-2">
						<button 
							class="btn variant-ghost-surface"
							on:click={resetChanges}
							disabled={saving}
						>
							Reset All Changes
						</button>
						<button 
							class="btn variant-filled-primary"
							on:click={handleSaveAll}
							disabled={saving}
						>
							{saving ? 'Saving...' : `Save ${Object.keys(pendingChanges).length} Changes`}
						</button>
					</div>
				</div>
			{/if}
		{/if}
	</div>
{:else}
	<Login />
{/if}