<script lang="ts">
import { User as UserIcon } from 'lucide-svelte';
import type { ComboboxItem, SearchResult, User } from '$lib/types/inaturalist';
import InatSearchChooser from './InatSearchChooser.svelte';

interface Props {
  selectedId?: number | string | null;
  onChange?: () => void;
  label?: string;
}

let { selectedId = $bindable(), onChange, label = 'User' }: Props = $props();

const mapUserResult = (result: SearchResult): ComboboxItem => {
  const user = (result.user || result.record) as User;
  return {
    label: user.login,
    value: user.id.toString(),
    item: user,
  };
};
</script>

<InatSearchChooser
  bind:selectedId
  {onChange}
  source="users"
  mapResultFn={mapUserResult}
  placeholder={label}
  {label}
>
  {#snippet thumbnail({ selectedItem })}
    {@const user = selectedItem as User | null}
    {#if user}
      {#if user.icon}
        <img
          src={user.icon.replace('thumb', 'medium')}
          alt={user.login}
          class="aspect-square h-9 object-cover rounded-full"
        />
      {:else}
        <div
          class="h-9 rounded-full aspect-square bg-surface-500 flex items-center justify-center text-surface-contrast-500"
        >
          <UserIcon size={16} />
        </div>
      {/if}
    {:else}
      <div
        class="h-9 rounded-full aspect-square bg-surface-200-800 flex items-center justify-center"
      >
        <UserIcon size={16} />
      </div>
    {/if}
  {/snippet}

  {#snippet itemContent({ item })}
    {@const user = item.item as User}
    <div class="flex w-full gap-2 items-center">
      {#if user?.icon}
        <img
          src={user.icon.replace('thumb', 'medium')}
          alt={user.login}
          class="h-9 rounded-full aspect-square object-cover"
        />
      {:else}
        <div
          class="h-9 rounded-full aspect-square bg-surface-200-800 flex items-center justify-center"
        >
          <UserIcon size={16} />
        </div>
      {/if}
      <div class="flex-1">
        <div class="line-clamp-1 text-ellipsis font-semibold">
          {user?.login || item.label}
        </div>
        {#if user?.name}
          <div class="line-clamp-1 text-ellipsis text-sm">
            {user.name}
          </div>
        {/if}
      </div>
    </div>
  {/snippet}
</InatSearchChooser>
