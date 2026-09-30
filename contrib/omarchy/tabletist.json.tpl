{
  "base": "{{ mode }}",
  "colors": {
    "window": "{{ background }}",
    "panel": "{{ dark_background }}",
    "surface": "{{ mix dark_background foreground 6% }}",
    "surface_hover": "{{ mix dark_background foreground 10% }}",
    "surface_active": "{{ mix dark_background foreground 16% }}",
    "outline": "{{ lighter_background }}",
    "text": "{{ foreground }}",
    "secondary": "{{ muted }}",
    "dim": "{{ muted }}",
    "faint": "{{ mix background muted 75% }}",
    "border": "{{ lighter_background }}",
    "accent": "{{ accent }}",
    "accent_hover": "{{ mix accent foreground 15% }}",
    "on_accent": "{{ background }}",
    "selection": "{{ selection }}",
    "danger": "{{ red }}",
    "warning": "{{ yellow }}",
    "success": "{{ green }}",
    "info": "{{ cyan }}",
    "orange": "{{ orange }}",
    "magenta": "{{ magenta }}"
  }
}
