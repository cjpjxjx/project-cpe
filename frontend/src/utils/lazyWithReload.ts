import { lazy, type ComponentType, type LazyExoticComponent } from 'react'

// 各浏览器对「模块取不到」的报错文案：Chrome / Safari / Firefox
const FETCH_FAILED = /failed to fetch dynamically imported module|importing a module script failed|error loading dynamically imported module/i

function isFetchFailure(error: unknown): boolean {
  return error instanceof Error && FETCH_FAILED.test(error.message)
}

/**
 * chunk 取不到时整页重载的 `React.lazy`。
 *
 * OTA 覆盖 www 后页面持有的是旧 chunk 文件名，重新请求同一个 URL 不会成功，
 * 只有重新加载 index.html 才能拿到新的 chunk 清单。模块已取到、执行期抛出的
 * 错误重载解决不了，原样抛给 ErrorBoundary。
 */
export function lazyWithReload(
  factory: () => Promise<{ default: ComponentType }>
): LazyExoticComponent<ComponentType> {
  return lazy(() =>
    factory().catch((error: unknown) => {
      if (!isFetchFailure(error)) {
        throw error
      }
      window.location.reload()
      // 重载期间保持挂起，不闪错误界面
      return new Promise<never>(() => {})
    })
  )
}
