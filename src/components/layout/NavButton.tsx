// src/components/layout/NavButton.tsx
import { Link, useLocation } from 'react-router-dom';

interface NavButtonProps {
  to: string;
  label: string;
  icon: React.ReactNode;
}

const NavButton = ({ to, label, icon }: NavButtonProps) => {
  const { pathname } = useLocation();
  const isActive = pathname === `/${to}` || pathname === '/' && to === '';

  return (
    <Link
      to={to === '' ? '/' : `/${to}`}
      className={`flex items-center gap-3 px-4 py-2.5 rounded-xl text-sm transition-all duration-150 ${
        isActive
          ? 'bg-white text-[#534AB7] font-medium shadow-sm'
          : 'text-slate-500 hover:text-slate-800 hover:bg-white/60'
      }`}
    >
      <span className={`text-base leading-none ${isActive ? 'text-[#534AB7]' : 'text-slate-400'}`}>
        {icon}
      </span>
      <span>{label}</span>
    </Link>
  );
};

export default NavButton;