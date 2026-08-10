import { isRouteErrorResponse, Link, useRouteError } from 'react-router-dom'
import { Button } from '@appica/ui-react/button'

/**
 * Route-level error boundary. Catches render/layout errors anywhere under
 * <App /> so the user sees a recoverable page instead of a blank crash.
 */
export default function ErrorPage() {
  const error = useRouteError()

  let detail = '页面加载出错了。'
  if (isRouteErrorResponse(error)) {
    detail = `${error.status} ${error.statusText}`
  } else if (error instanceof Error) {
    detail = error.message
  }

  return (
    <div className="flex h-full flex-col items-center justify-center gap-4 px-6 text-center">
      <h1 className="text-lg font-semibold">出错了</h1>
      <p className="text-foreground-muted text-sm">{detail}</p>
      <div className="flex items-center gap-2">
        <Button onClick={() => window.location.reload()}>重新加载</Button>
        <Link to="/">
          <Button variant="ghost">回到首页</Button>
        </Link>
      </div>
    </div>
  )
}
