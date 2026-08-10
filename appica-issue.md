### Title: `Field` throws `FieldRootContext is missing` (production error #28) with `@base-ui/react` 1.7.x — the `^1.6.0` range resolves to an incompatible 1.7.0

**Package version**: @appica/ui-react 1.0.0
**Environment**: React 19.1, Vite 6, Tailwind v4, npm (Windows)

### Summary

`@appica/ui-react` declares `@base-ui/react: ^1.6.0` as a dependency. A fresh `npm install` today resolves that range to **1.7.0**, under which the `Field` compound breaks at runtime:

```
Base UI error #28; visit https://base-ui.com/production-error?code=28
Base UI: FieldRootContext is missing. Field parts must be placed within <Field.Root>.
The above error occurred in the <FieldDescription> component.
```

Any screen rendering `<Field>` + `<FieldLabel>`/`<FieldDescription>`/`<FieldError>` (and form components built on Field, e.g. `Input`, `Select`, `Textarea` inside a Field) crashes the React tree. In our app this took down the whole settings/admin panels with production error #28 (Field) — and a related #51 on another tab that also renders Field-based forms.

### Reproduction

```tsx
import { Field, FieldLabel, FieldDescription } from '@appica/ui-react/field'
import { Input } from '@appica/ui-react/input'

export function Demo() {
  return (
    <Field>
      <FieldLabel>Name</FieldLabel>
      <Input />
      <FieldDescription>Any description</FieldDescription>
    </Field>
  )
}
```

With `@base-ui/react@1.7.0` → render error `FieldRootContext is missing`. Pinning `@base-ui/react@1.6.0` (the exact version the caret range was authored against) resolves it.

### Impact

- Production builds minify the message to `Base UI error #28` — no context, hard to diagnose.
- Any app that installs today gets a broken Field out of the box until the base-ui version is pinned manually.

### Suggested fixes (any would help)

1. Tighten the dependency range to `~1.6.0` (or an upper bound) so future base-ui majors/minors can't silently break Field.
2. Or fix the Field implementation for `@base-ui/react` 1.7.x (the 1.7 `FieldRoot` restructured to `LabelableProvider > FieldRootInner` — the context should still be provided, but something in the interplay with Appica's wrapper breaks it).
3. Or document the required base-ui version as a peer/overrides entry.

### Workaround for now

```json
// package.json overrides
"overrides": { "@base-ui/react": "1.6.0" }
```
or `npm i @base-ui/react@1.6.0`.
