import { Skeleton } from '@/components/ui'

export type TradePageWorkspaceSkeletonProps = {
  /** When true, include Trade page title + pair selector placeholders (route chunk fallback). */
  includePageChrome?: boolean
}

/**
 * Placeholder for chart / book / ticket while trade data or the route chunk loads (GitLab #179).
 * Sized so main content paints before the legal footer for LCP.
 */
export function TradePageWorkspaceSkeleton({ includePageChrome = false }: TradePageWorkspaceSkeletonProps) {
  return (
    <div
      className="space-y-3"
      data-testid="trade-workspace-skeleton"
      role="status"
      aria-busy="true"
      aria-label="Loading trade workspace"
    >
      {includePageChrome ? (
        <>
          <div className="space-y-2">
            <Skeleton height="1.5rem" width="6rem" />
            <Skeleton height="0.875rem" width="min(100%, 28rem)" />
          </div>
          <div className="shell-panel p-3 space-y-2">
            <Skeleton height="0.75rem" width="3rem" />
            <Skeleton height="2.5rem" width="100%" className="max-w-xl" />
          </div>
        </>
      ) : null}

      <div
        className="lg:hidden grid grid-cols-1 gap-3 md:grid-cols-2 min-h-[min(72vh,640px)]"
        data-testid="trade-workspace-skeleton-sub-lg"
      >
        <div className="min-h-[280px] md:col-span-2 md:row-start-2 card-neo !p-3 flex flex-col gap-2">
          <Skeleton height="0.75rem" width="5rem" />
          <Skeleton height="100%" className="flex-1 min-h-[220px]" />
        </div>
        <div className="min-h-[200px] md:col-start-2 md:row-start-1 card-neo !p-3 flex flex-col gap-2">
          <Skeleton height="0.75rem" width="6rem" />
          <Skeleton height="100%" className="flex-1 min-h-[160px]" />
        </div>
        <div className="min-h-[220px] md:min-h-[280px] md:col-start-1 md:row-start-1 card-neo !p-2 flex flex-col gap-2">
          <Skeleton height="0.75rem" width="5rem" />
          <Skeleton height="100%" className="flex-1 min-h-[180px]" />
        </div>
      </div>

      <div
        className="hidden lg:grid gap-3 min-h-[min(72vh,640px)] h-[min(85vh,920px)]"
        style={{ gridTemplateColumns: '24% 52% 24%' }}
        data-testid="trade-workspace-skeleton-desktop"
      >
        <div className="card-neo !p-3 flex flex-col gap-2 min-h-0">
          <Skeleton height="0.75rem" width="5rem" />
          <Skeleton height="100%" className="flex-1 min-h-[320px]" />
        </div>
        <div className="flex flex-col gap-3 min-h-0">
          <div className="card-neo !p-2 flex flex-col gap-2 flex-[1.4] min-h-[240px]">
            <Skeleton height="0.75rem" width="5rem" />
            <Skeleton height="100%" className="flex-1" />
          </div>
          <div className="card-neo !p-3 flex flex-col gap-2 flex-1 min-h-[140px]">
            <Skeleton height="0.75rem" width="6rem" />
            <Skeleton height="100%" className="flex-1" />
          </div>
        </div>
        <div className="card-neo !p-3 flex flex-col gap-2 min-h-0">
          <Skeleton height="0.75rem" width="6rem" />
          <Skeleton height="100%" className="flex-1 min-h-[320px]" />
        </div>
      </div>
    </div>
  )
}
