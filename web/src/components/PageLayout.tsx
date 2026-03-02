import type { ReactNode } from "react";

interface PageLayoutProps {
  title: string;
  subtitle?: ReactNode;
  icon?: ReactNode;
  action?: ReactNode;
  children: ReactNode;
}

export default function PageLayout({
  title,
  subtitle,
  icon,
  action,
  children,
}: PageLayoutProps) {
  return (
    <div className="p-4 sm:p-6 lg:p-8 max-w-screen-2xl mx-auto">
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between mb-6 gap-3">
        <div>
          <h1 className="text-xl sm:text-2xl font-bold text-claw-100 flex items-center gap-2">
            {icon}
            {title}
          </h1>
          {subtitle && (
            <div className="text-sm text-gray-400 mt-1">{subtitle}</div>
          )}
        </div>
        {action && <div className="shrink-0">{action}</div>}
      </div>
      {children}
    </div>
  );
}
