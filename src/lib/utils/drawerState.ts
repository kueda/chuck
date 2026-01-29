import type { Occurrence } from '$lib/types/archive';

export interface DrawerState {
  open: boolean;
  selectedOccurrenceId: string | number | null;
  selectedOccurrenceIndex: number | null;
}

export interface DrawerHandlers {
  handleOccurrenceClick: (occurrence: Occurrence, index: number) => void;
  handleClose: () => void;
  handlePrevious?: () => void;
  handleNext?: () => void;
}

export function createDrawerHandlers(options: {
  state: DrawerState;
  occurrenceCache: Map<number, Occurrence>;
  coreIdColumn: string;
  count: number;
  scrollToIndex?: (
    index: number,
    options?: { align?: 'start' | 'center' | 'end' | 'auto' },
  ) => void;
}): DrawerHandlers {
  const { state, occurrenceCache, coreIdColumn, count, scrollToIndex } =
    options;

  const handleOccurrenceClick = (occurrence: Occurrence, index: number) => {
    const value = occurrence[coreIdColumn as keyof Occurrence];
    state.selectedOccurrenceId =
      typeof value === 'string' || typeof value === 'number' ? value : null;
    state.selectedOccurrenceIndex = index;
    state.open = true;
  };

  const handleClose = () => {
    state.open = false;
  };

  // Only provide navigation handlers if scrollToIndex is available
  let handlePrevious: (() => void) | undefined;
  let handleNext: (() => void) | undefined;

  if (scrollToIndex && state.selectedOccurrenceIndex !== null) {
    if (state.selectedOccurrenceIndex > 0) {
      handlePrevious = () => {
        const newIndex = (state.selectedOccurrenceIndex ?? 0) - 1;
        const prevOccurrence = occurrenceCache.get(newIndex);
        if (prevOccurrence) {
          const value = prevOccurrence[coreIdColumn as keyof Occurrence];
          state.selectedOccurrenceId =
            typeof value === 'string' || typeof value === 'number'
              ? value
              : null;
          state.selectedOccurrenceIndex = newIndex;
        }
        scrollToIndex(state.selectedOccurrenceIndex ?? 0, { align: 'auto' });
      };
    }

    if (state.selectedOccurrenceIndex < count - 1) {
      handleNext = () => {
        const newIndex = (state.selectedOccurrenceIndex ?? 0) + 1;
        const nextOccurrence = occurrenceCache.get(newIndex);
        if (nextOccurrence) {
          const value = nextOccurrence[coreIdColumn as keyof Occurrence];
          state.selectedOccurrenceId =
            typeof value === 'string' || typeof value === 'number'
              ? value
              : null;
          state.selectedOccurrenceIndex = newIndex;
        }
        // Scroll past current item when it's at the bottom
        scrollToIndex((state.selectedOccurrenceIndex ?? 0) + 1, {
          align: 'auto',
        });
      };
    }
  }

  return {
    handleOccurrenceClick,
    handleClose,
    handlePrevious,
    handleNext,
  };
}
