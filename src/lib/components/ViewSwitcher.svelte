<script lang="ts">
  import { SegmentedControl } from '@skeletonlabs/skeleton-svelte';
  import { BookImage, MapIcon, Sheet } from 'lucide-svelte';

  let {
    view = $bindable('table'),
    onViewChange
  }: {
    view: string,
    onViewChange: () => void
  } = $props();
</script>

<SegmentedControl
  value={view}
  onValueChange={(e) => {
    onViewChange();
    return view = e.value || 'table';
  }}
>
  <SegmentedControl.Control class="bg-white shadow-lg">
    <SegmentedControl.Indicator />
    <SegmentedControl.Item value="table">
      <SegmentedControl.ItemText>
        <div class="flex flex-row gap-1 justify-center items-center">
          <Sheet size={20} />
          Table
        </div>
      </SegmentedControl.ItemText>
      <SegmentedControl.ItemHiddenInput />
    </SegmentedControl.Item>
    <SegmentedControl.Item value="cards">
      <SegmentedControl.ItemText>
        <div class="flex flex-row gap-1 justify-center items-center">
          <BookImage size={20} />
          Cards
        </div>
      </SegmentedControl.ItemText>
      <SegmentedControl.ItemHiddenInput />
    </SegmentedControl.Item>
    <SegmentedControl.Item value="map">
      <SegmentedControl.ItemText>
        <div class="flex flex-row gap-1 justify-center items-center">
          <MapIcon size={20} />
          Map
        </div>
      </SegmentedControl.ItemText>
      <SegmentedControl.ItemHiddenInput />
    </SegmentedControl.Item>
  </SegmentedControl.Control>
</SegmentedControl>
