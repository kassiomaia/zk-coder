/**
 * Collection Utilities
 * Generic helper functions for array operations on collections.
 */

/**
 * Finds an item in an array by its ID property.
 */
export function findById<T extends { id: string }>(
	items: T[],
	id: string
): T | undefined {
	return items.find((item) => item.id === id);
}

/**
 * Updates an item in an array by ID.
 * Returns a new array with the matching item replaced.
 */
export function updateById<T extends { id: string }>(
	items: T[],
	id: string,
	updates: Partial<T>
): T[] {
	return items.map((item) =>
		item.id === id ? { ...item, ...updates } : item
	);
}

/**
 * Removes an item from an array by ID.
 * Returns a new array without the matching item.
 */
export function removeById<T extends { id: string }>(
	items: T[],
	id: string
): T[] {
	return items.filter((item) => item.id !== id);
}

/**
 * Upserts an item in an array (insert if not exists, update if exists).
 * Returns a new array.
 */
export function upsertById<T extends { id: string }>(
	items: T[],
	item: T
): T[] {
	const exists = items.some((i) => i.id === item.id);
	if (exists) {
		return items.map((i) => (i.id === item.id ? item : i));
	}
	return [...items, item];
}

/**
 * Filters an array to exclude a specific ID.
 */
export function excludeById<T extends { id: string }>(
	items: T[],
	id: string
): T[] {
	return items.filter((item) => item.id !== id);
}

/**
 * Filters an array to exclude multiple IDs.
 */
export function excludeByIds<T extends { id: string }>(
	items: T[],
	ids: string[]
): T[] {
	const idSet = new Set(ids);
	return items.filter((item) => !idSet.has(item.id));
}

/**
 * Updates nested array items (e.g., threads within conversations).
 * Generic helper for updating items in a parent collection's child arrays.
 */
export function updateNestedById<TParent extends { id: string }, TChild extends { id: string }>(
	parentArray: TParent[],
	parentId: string,
	childKey: keyof TParent,
	childId: string,
	childUpdates: Partial<TChild>
): TParent[] {
	return parentArray.map((parent) => {
		if (parent.id !== parentId) return parent;

		const children = parent[childKey];
		if (!Array.isArray(children)) return parent;

		const updatedChildren = children.map((child: TChild) =>
			child.id === childId ? { ...child, ...childUpdates } : child
		);

		return { ...parent, [childKey]: updatedChildren };
	});
}

/**
 * Removes from nested array items.
 */
export function removeNestedById<TParent extends { id: string }, TChild extends { id: string }>(
	parentArray: TParent[],
	parentId: string,
	childKey: keyof TParent,
	childId: string
): TParent[] {
	return parentArray.map((parent) => {
		if (parent.id !== parentId) return parent;

		const children = parent[childKey];
		if (!Array.isArray(children)) return parent;

		const filteredChildren = (children as TChild[]).filter(
			(child) => child.id !== childId
		);

		return { ...parent, [childKey]: filteredChildren };
	});
}