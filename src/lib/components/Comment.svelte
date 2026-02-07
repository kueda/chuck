<script lang="ts">
import ActivityCard from '$lib/components/ActivityCard.svelte';
import Markup from '$lib/components/Markup.svelte';
import type { Comment } from '$lib/types/archive';

interface Props {
  comment: Comment;
}

const { comment }: Props = $props();
</script>

<ActivityCard
  agentName={comment.author}
  agentId={comment.authorID}
  date={comment.created}
>
  {#snippet children()}
    <Markup text={comment.text.toString()} />
  {/snippet}
</ActivityCard>
