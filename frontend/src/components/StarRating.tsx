// AOS-P2 — Star rating display
import { Star } from 'lucide-react';

interface StarRatingProps {
  rating: number;
  max?: number;
  size?: number;
  className?: string;
}

export default function StarRating({ rating, max = 5, size = 14, className = '' }: StarRatingProps) {
  return (
    <span className={`inline-flex items-center gap-0.5 ${className}`}>
      {Array.from({ length: max }, (_, i) => (
        <Star
          key={i}
          size={size}
          className={i < rating ? 'text-[#F39C12] fill-[#F39C12]' : 'text-[#1A1E26]'}
        />
      ))}
    </span>
  );
}
