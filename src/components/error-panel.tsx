import { AlertTriangleIcon, ChevronRightIcon } from "lucide-react";

import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { cn } from "@/lib/utils";
import { toAppError } from "@/types/errors";

interface ErrorPanelProps {
  error: unknown;
  className?: string;
}

/**
 * Renders a backend failure as an actionable sentence with the raw tool output kept one click
 * away. The raw text is never discarded or rewritten.
 */
export function ErrorPanel({ error, className }: ErrorPanelProps) {
  const appError = toAppError(error);

  return (
    <Alert variant="destructive" className={className}>
      <AlertTriangleIcon />
      <AlertTitle>{appError.message}</AlertTitle>
      <AlertDescription className="flex flex-col items-start gap-2">
        {appError.hint ? <span>{appError.hint}</span> : null}
        {appError.details ? (
          <Collapsible className="w-full">
            <CollapsibleTrigger className="group flex items-center gap-1 text-xs font-medium underline-offset-2 hover:underline">
              <ChevronRightIcon
                className={cn(
                  "size-3 transition-transform",
                  "group-data-[state=open]:rotate-90",
                )}
              />
              Raw output
            </CollapsibleTrigger>
            <CollapsibleContent>
              <pre className="mt-2 max-h-56 overflow-auto rounded-md bg-muted p-3 font-mono text-xs whitespace-pre-wrap text-muted-foreground">
                {appError.details}
              </pre>
            </CollapsibleContent>
          </Collapsible>
        ) : null}
      </AlertDescription>
    </Alert>
  );
}
