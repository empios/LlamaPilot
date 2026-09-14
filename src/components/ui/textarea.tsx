import * as React from "react"

import { cn } from "@/lib/utils"

function Textarea({ className, ...props }: React.ComponentProps<"textarea">) {
  return (
    <textarea
      data-slot="textarea"
      className={cn(
        "flex field-sizing-content min-h-20 w-full rounded-md border border-input border-b-[var(--pg-border-strong-01)] bg-[var(--pg-field-01)] px-3 py-2 text-base transition-colors duration-100 outline-none placeholder:text-[var(--pg-text-placeholder)] hover:bg-[var(--pg-field-hover)] focus-visible:border-ring focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:bg-muted disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-1 aria-invalid:ring-destructive md:text-sm",
        className
      )}
      {...props}
    />
  )
}

export { Textarea }
