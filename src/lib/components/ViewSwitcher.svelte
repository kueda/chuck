<script lang="ts">
import { SegmentedControl } from '@skeletonlabs/skeleton-svelte';
import { BookImage, MapIcon, Rows3, Sheet } from 'lucide-svelte';

let {
  view = $bindable('table'),
  views = ['table', 'cards', 'map'],
  onViewChange,
}: {
  view: string;
  views?: ('table' | 'cards' | 'map' | 'rows')[];
  onViewChange?: () => void;
} = $props();
</script>

<SegmentedControl
  value={view}
  onValueChange={(e) => {
    if (typeof (onViewChange) === "function") onViewChange();
    return view = e.value || 'table';
  }}
>
  <SegmentedControl.Control class="bg-white shadow-lg">
    <SegmentedControl.Indicator />
    {#if views.includes('table')}
      <SegmentedControl.Item value="table">
        <SegmentedControl.ItemText>
          <div class="flex flex-row gap-1 justify-center items-center">
            <Sheet size={20} />
            Table
          </div>
        </SegmentedControl.ItemText>
        <SegmentedControl.ItemHiddenInput />
      </SegmentedControl.Item>
    {/if}
    {#if views.includes('cards')}
      <SegmentedControl.Item value="cards">
        <SegmentedControl.ItemText>
          <div class="flex flex-row gap-1 justify-center items-center">
            <BookImage size={20} />
            Cards
          </div>
        </SegmentedControl.ItemText>
        <SegmentedControl.ItemHiddenInput />
      </SegmentedControl.Item>
    {/if}
    {#if views.includes('map')}
      <SegmentedControl.Item value="map">
        <SegmentedControl.ItemText>
          <div class="flex flex-row gap-1 justify-center items-center">
            <MapIcon size={20} />
            Map
          </div>
        </SegmentedControl.ItemText>
        <SegmentedControl.ItemHiddenInput />
      </SegmentedControl.Item>
    {/if}
    {#if views.includes('rows')}
      <SegmentedControl.Item value="rows">
        <SegmentedControl.ItemText>
          <div class="flex flex-row gap-1 justify-center items-center">
            <Rows3 size={20} />
            Rows
          </div>
        </SegmentedControl.ItemText>
        <SegmentedControl.ItemHiddenInput />
      </SegmentedControl.Item>
    {/if}
    </SegmentedControl.Control>
</SegmentedControl>
