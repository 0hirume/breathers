from __future__ import annotations

import example

from .other import (
    first,
)
from .third import second

if TYPE_CHECKING:
    from .other import (
        Example,
    )
    from .third import Other
